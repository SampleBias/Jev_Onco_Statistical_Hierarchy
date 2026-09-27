//! Model-agnostic probability-space Shapley values. No trained surrogate model.
pub mod diagnostics;
use josh_core::{
    JevRequest, JevResponse, ValidationError,
    molecular::{self, FeatureSet, InferenceRun, MeasurementStatus, Modality},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    future::Future,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Invalid(#[from] ValidationError),
    #[error("explanation evaluation budget exhausted; completed calls are retained")]
    Budget,
    #[error("explanation cancelled; completed calls are retained")]
    Cancelled,
    #[error("provider evaluation failed; no attribution was substituted")]
    Provider,
    #[error("provider returned HTTP {0}; completed calls are retained")]
    ProviderHttp(u16),
    #[error("{0}; completed calls are retained")]
    ProviderResponse(Box<josh_core::response::ResponseDiagnostic>),
    #[error("explanation wall-time limit reached; completed calls are retained")]
    Timeout,
    #[error("archive checkpoint failed")]
    Checkpoint,
    #[error(
        "previous evaluation completion is uncertain; explicit --retry-uncertain is required before resending"
    )]
    Uncertain,
}

pub trait Evaluator: Send {
    fn evaluate(
        &mut self,
        request: &JevRequest,
    ) -> impl Future<Output = Result<JevResponse, Error>> + Send;
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Algorithm {
    Exact,
    Permutation,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Baseline {
    MaskedEvidence,
    ReferenceBackground,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub algorithm: Algorithm,
    pub baseline: Baseline,
    pub target_class: String,
    pub permutation_pairs: usize,
    pub seed: u64,
    pub max_evaluations: usize,
    pub max_input_tokens: u64,
    pub max_seconds: u64,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            algorithm: Algorithm::Permutation,
            baseline: Baseline::MaskedEvidence,
            target_class: "NSCLC".into(),
            permutation_pairs: 32,
            seed: 42,
            max_evaluations: 1024,
            max_input_tokens: 2_000_000,
            max_seconds: 600,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Background {
    pub release_id: String,
    pub partition: String,
    pub samples: Vec<FeatureSet>,
}
impl Background {
    fn validate(&self, query: &FeatureSet) -> Result<(), ValidationError> {
        if self.partition != "development"
            || self.release_id.trim().is_empty()
            || self.release_id.len() > 128
            || self.samples.is_empty()
            || self.samples.len() > 64
        {
            return Err(ValidationError(
                "background must be a bounded development reference",
            ));
        }
        let mut patients = BTreeSet::new();
        for b in &self.samples {
            b.validate()?;
            if b.sample_id == query.sample_id
                || b.patient_group_id == query.patient_group_id
                || !patients.insert(&b.patient_group_id)
                || b.data_class != query.data_class
                || b.features.len() != query.features.len()
            {
                return Err(ValidationError(
                    "background overlaps query, duplicates patients or is incompatible",
                ));
            }
            for q in &query.features {
                let Some(f) = b.features.iter().find(|f| f.id == q.id) else {
                    return Err(ValidationError("background feature IDs differ"));
                };
                if f.name != q.name
                    || f.modality != q.modality
                    || f.group != q.group
                    || f.assay != q.assay
                    || f.reference_build != q.reference_build
                    || f.coverage != q.coverage
                    || f.depends_on != q.depends_on
                    || f.status != q.status
                    || f.value.as_ref().map(|v| v.encoding())
                        != q.value.as_ref().map(|v| v.encoding())
                {
                    return Err(ValidationError(
                        "background encodings, coverage or groups are incompatible",
                    ));
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Evaluation {
    pub request: JevRequest,
    pub response: JevResponse,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Attribution {
    pub group: String,
    pub feature_ids: Vec<String>,
    pub label: String,
    pub modalities: Vec<Modality>,
    pub contribution: f64,
    /// Monte Carlo standard error over paired permutations; not a clinical interval.
    pub sampling_standard_error: Option<f64>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExplanationResult {
    pub method: String,
    pub target_class: String,
    pub baseline_probability: f64,
    pub full_probability: f64,
    pub attributions: Vec<Attribution>,
    pub additivity_residual: f64,
    pub evaluations: usize,
    pub input_tokens: u64,
    pub background_sha256: Option<String>,
    pub limitations: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Archive {
    pub schema_version: u32,
    pub features: FeatureSet,
    pub inference: InferenceRun,
    pub config: Config,
    pub background: Option<Background>,
    pub evaluations: BTreeMap<String, Evaluation>,
    pub attempts: usize,
    pub uncertain_input_tokens: u64,
    pub in_flight: Option<JevRequest>,
    pub result: Option<ExplanationResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Plan {
    pub groups: Vec<String>,
    pub coalition_upper_bound: usize,
    pub evaluation_upper_bound: usize,
    pub budget: usize,
    pub fits_budget: bool,
    pub baseline: Baseline,
}

impl Archive {
    pub fn new(
        features: FeatureSet,
        inference: InferenceRun,
        config: Config,
        background: Option<Background>,
    ) -> Result<Self, Error> {
        let mut archive = Self {
            schema_version: 1,
            features,
            inference,
            config,
            background,
            evaluations: BTreeMap::new(),
            attempts: 1,
            uncertain_input_tokens: 0,
            in_flight: None,
            result: None,
        };
        archive.evaluations.insert(
            archive.inference.request_sha256.clone(),
            Evaluation {
                request: archive.inference.request.clone(),
                response: archive.inference.response.clone(),
            },
        );
        archive.validate()?;
        Ok(archive)
    }
    pub fn validate(&self) -> Result<(), Error> {
        self.inference.verify(&self.features)?;
        let groups = self.features.groups();
        if self.schema_version != 1
            || groups.is_empty()
            || groups.len() > 512
            || self.config.max_evaluations == 0
            || self.config.max_evaluations > 100_000
            || self.config.max_input_tokens == 0
            || self.config.max_input_tokens > 100_000_000
            || self.config.max_seconds == 0
            || self.config.max_seconds > 86_400
            || self.config.permutation_pairs == 0
            || self.config.permutation_pairs > 10_000
            || !self
                .inference
                .taxonomy
                .criteria()
                .contains_key(&self.config.target_class)
            || (self.config.algorithm == Algorithm::Permutation
                && self
                    .config
                    .permutation_pairs
                    .saturating_mul(2)
                    .saturating_mul(groups.len() + 1)
                    > 100_000)
            || (self.config.algorithm == Algorithm::Exact && groups.len() > 16)
        {
            return Err(ValidationError(
                "invalid explanation configuration; exact enumeration supports at most 16 groups",
            )
            .into());
        }
        match (&self.config.baseline, &self.background) {
            (Baseline::MaskedEvidence, None) => (),
            (Baseline::ReferenceBackground, Some(b)) => b.validate(&self.features)?,
            _ => {
                return Err(
                    ValidationError("baseline and background configuration disagree").into(),
                );
            }
        }
        if self.attempts > 100_000 || self.attempts < self.evaluations.len() {
            return Err(ValidationError("invalid evaluation attempt accounting").into());
        }
        for (key, e) in &self.evaluations {
            if key != &molecular::hash(&e.request)?
                || !molecular::explanation_questions_match(
                    &e.request.questions,
                    &self.inference.request.questions,
                )?
            {
                return Err(ValidationError("cached request mismatch").into());
            }
            molecular::validate_response(&e.request, &e.response)?;
        }
        let full = self
            .evaluations
            .get(&self.inference.request_sha256)
            .ok_or(ValidationError("archive lacks original inference"))?;
        if molecular::hash(&full.response)? != molecular::hash(&self.inference.response)? {
            return Err(ValidationError("cached full output differs from inference").into());
        }
        if let Some(pending) = &self.in_flight
            && (!molecular::explanation_questions_match(
                &pending.questions,
                &self.inference.request.questions,
            )? || self.evaluations.contains_key(&molecular::hash(pending)?)
                || self.result.is_some())
        {
            return Err(ValidationError("invalid pending provider evaluation").into());
        }
        // A provider may report more tokens than reserved. The engine stops new
        // calls, but the archive must remain inspectable and resumable with a
        // consciously increased budget.
        if let Some(result) = &self.result {
            let expected = calculate(self)?;
            if molecular::hash(result)? != molecular::hash(&expected)? {
                return Err(ValidationError(
                    "attribution archive does not reproduce from stored evaluations",
                )
                .into());
            }
        }
        Ok(())
    }
    /// Reuse complete coalition distributions to explain another class offline.
    pub fn retarget(&mut self, target: &str) -> Result<(), Error> {
        self.validate()?;
        if !self.inference.taxonomy.criteria().contains_key(target) || self.result.is_none() {
            return Err(ValidationError(
                "retargeting requires a complete archive and a supplied class",
            )
            .into());
        }
        self.config.target_class = target.into();
        self.result = Some(calculate(self)?);
        Ok(())
    }
    pub fn plan(&self) -> Plan {
        let groups = self.features.groups();
        let n = groups.len();
        let coalitions = if self.config.algorithm == Algorithm::Exact {
            1usize.checked_shl(n as u32).unwrap_or(usize::MAX)
        } else {
            self.config
                .permutation_pairs
                .saturating_mul(2)
                .saturating_mul(n + 1)
        };
        let evaluations =
            coalitions.saturating_mul(self.background.as_ref().map_or(1, |b| b.samples.len()));
        Plan {
            groups,
            coalition_upper_bound: coalitions,
            evaluation_upper_bound: evaluations,
            budget: self.config.max_evaluations,
            fits_budget: evaluations <= self.config.max_evaluations,
            baseline: self.config.baseline,
        }
    }
    pub fn tokens(&self) -> u64 {
        self.evaluations
            .values()
            .map(|e| e.response.usage.input_tokens)
            .fold(0, u64::saturating_add)
    }
    fn requests(&self, visible: &BTreeSet<String>) -> Result<Vec<JevRequest>, Error> {
        if let Some(background) = &self.background {
            background
                .samples
                .iter()
                .map(|b| {
                    let mut hybrid = self.features.clone();
                    for f in &mut hybrid.features {
                        if !visible.contains(&f.group) && f.status == MeasurementStatus::Observed {
                            f.value = b
                                .features
                                .iter()
                                .find(|x| x.id == f.id)
                                .expect("validated background")
                                .value
                                .clone();
                        }
                    }
                    Ok(molecular::prepare_masked_versioned(
                        &hybrid,
                        &self.inference.taxonomy,
                        &hybrid.groups().into_iter().collect(),
                        &self.inference.prompt_version,
                    )?)
                })
                .collect()
        } else {
            Ok(vec![molecular::prepare_masked_versioned(
                &self.features,
                &self.inference.taxonomy,
                visible,
                &self.inference.prompt_version,
            )?])
        }
    }
    fn value(&self, visible: &BTreeSet<String>) -> Result<f64, Error> {
        let requests = self.requests(visible)?;
        let mut sum = 0.0;
        for request in &requests {
            let key = molecular::hash(request)?;
            let evaluation = self
                .evaluations
                .get(&key)
                .ok_or(ValidationError("incomplete coalition evaluations"))?;
            sum += molecular::probabilities(&evaluation.response)?[&self.config.target_class];
        }
        Ok(sum / requests.len() as f64)
    }
}

#[derive(Clone, Default)]
pub struct Progress {
    pub cancelled: Arc<AtomicBool>,
    pub completed: Arc<AtomicUsize>,
}

/// Checkpoint receives the complete archive after each successful response. On errors
/// the caller still owns all successful evaluations; no partial attribution is published.
pub async fn run<E: Evaluator>(
    archive: &mut Archive,
    evaluator: &mut E,
    progress: &Progress,
    mut checkpoint: impl FnMut(&Archive) -> Result<(), Error>,
) -> Result<(), Error> {
    archive.validate()?;
    archive.result = None;
    if archive.in_flight.is_some() {
        return Err(Error::Uncertain);
    }
    let started = Instant::now();
    let duration = Duration::from_secs(archive.config.max_seconds);
    let groups = archive.features.groups();
    let masks = masks(&groups, &archive.config);
    for visible in masks {
        if progress.cancelled.load(Ordering::Relaxed) {
            return Err(Error::Cancelled);
        }
        if started.elapsed() >= duration {
            return Err(Error::Timeout);
        }
        for request in archive.requests(&visible)? {
            if progress.cancelled.load(Ordering::Relaxed) {
                return Err(Error::Cancelled);
            }
            let key = molecular::hash(&request)?;
            if archive.evaluations.contains_key(&key) {
                continue;
            }
            // UTF-8 bytes are a conservative request input-token reservation.
            let reservation = serde_json::to_vec(&request)
                .map_err(|_| ValidationError("request serialization"))?
                .len() as u64;
            if archive.attempts >= archive.config.max_evaluations
                || archive
                    .tokens()
                    .saturating_add(archive.uncertain_input_tokens)
                    .saturating_add(reservation)
                    > archive.config.max_input_tokens
            {
                return Err(Error::Budget);
            }
            let remaining = duration
                .checked_sub(started.elapsed())
                .ok_or(Error::Timeout)?;
            archive.attempts += 1;
            archive.uncertain_input_tokens =
                archive.uncertain_input_tokens.saturating_add(reservation);
            archive.in_flight = Some(request.clone());
            checkpoint(archive)?;
            let response = tokio::time::timeout(remaining, evaluator.evaluate(&request))
                .await
                .map_err(|_| Error::Timeout)??;
            molecular::validate_response(&request, &response)?;
            archive.in_flight = None;
            archive.uncertain_input_tokens =
                archive.uncertain_input_tokens.saturating_sub(reservation);
            archive
                .evaluations
                .insert(key, Evaluation { request, response });
            progress
                .completed
                .store(archive.evaluations.len(), Ordering::Relaxed);
            checkpoint(archive)?;
            if archive.tokens() > archive.config.max_input_tokens {
                return Err(Error::Budget);
            }
        }
    }
    archive.result = Some(calculate(archive)?);
    checkpoint(archive)?;
    Ok(())
}

fn permutations(n: usize, pairs: usize, seed: u64) -> Vec<Vec<usize>> {
    let mut state = seed;
    let mut random = || {
        state = state.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    };
    (0..pairs)
        .map(|_| {
            let mut order: Vec<_> = (0..n).collect();
            for i in (1..n).rev() {
                let j = (random() % (i as u64 + 1)) as usize;
                order.swap(i, j);
            }
            order
        })
        .collect()
}
fn masks<'a>(
    groups: &'a [String],
    config: &Config,
) -> Box<dyn Iterator<Item = BTreeSet<String>> + Send + 'a> {
    if config.algorithm == Algorithm::Exact {
        Box::new((0..(1usize << groups.len())).map(|bits| {
            groups
                .iter()
                .enumerate()
                .filter(|(i, _)| bits & (1 << i) != 0)
                .map(|(_, g)| g.clone())
                .collect()
        }))
    } else {
        Box::new(
            std::iter::once(BTreeSet::new()).chain(
                permutations(groups.len(), config.permutation_pairs, config.seed)
                    .into_iter()
                    .flat_map(move |order| {
                        [order.clone(), order.into_iter().rev().collect()]
                            .into_iter()
                            .flat_map(move |direction| {
                                direction
                                    .into_iter()
                                    .scan(BTreeSet::new(), move |visible, i| {
                                        visible.insert(groups[i].clone());
                                        Some(visible.clone())
                                    })
                            })
                    }),
            ),
        )
    }
}
fn choose(n: usize, k: usize) -> f64 {
    (0..k.min(n - k)).fold(1.0, |v, i| v * (n - i) as f64 / (i + 1) as f64)
}
fn calculate(archive: &Archive) -> Result<ExplanationResult, Error> {
    let groups = archive.features.groups();
    let n = groups.len();
    let mut values = vec![0.0; n];
    let mut errors = vec![None; n];
    let empty = BTreeSet::new();
    let baseline = archive.value(&empty)?;
    let full = archive.value(&groups.iter().cloned().collect())?;
    if archive.config.algorithm == Algorithm::Exact {
        let coalition_values = masks(&groups, &archive.config)
            .map(|s| archive.value(&s))
            .collect::<Result<Vec<_>, _>>()?;
        for (i, value) in values.iter_mut().enumerate() {
            for bits in 0..coalition_values.len() {
                if bits & (1 << i) != 0 {
                    continue;
                }
                let k = bits.count_ones() as usize;
                *value += (coalition_values[bits | (1 << i)] - coalition_values[bits])
                    / (n as f64 * choose(n - 1, k));
            }
        }
    } else {
        let pairs = archive.config.permutation_pairs;
        let mut means = vec![0.0; n];
        let mut m2 = vec![0.0; n];
        for (sample, order) in permutations(n, pairs, archive.config.seed)
            .into_iter()
            .enumerate()
        {
            let mut pair = vec![0.0; n];
            for direction in [order.clone(), order.into_iter().rev().collect()] {
                let mut s = BTreeSet::new();
                let mut previous = baseline;
                for i in direction {
                    s.insert(groups[i].clone());
                    let next = archive.value(&s)?;
                    pair[i] += (next - previous) / 2.0;
                    previous = next;
                }
            }
            for i in 0..n {
                let delta = pair[i] - means[i];
                means[i] += delta / (sample + 1) as f64;
                m2[i] += delta * (pair[i] - means[i]);
            }
        }
        values = means;
        if pairs > 1 {
            for i in 0..n {
                errors[i] = Some((m2[i].max(0.0) / ((pairs - 1) * pairs) as f64).sqrt());
            }
        }
    }
    let residual = full - baseline - values.iter().sum::<f64>();
    if !residual.is_finite() || residual.abs() > 1e-8 {
        return Err(ValidationError("Shapley additivity failed").into());
    }
    let attributions = groups
        .iter()
        .enumerate()
        .map(|(i, g)| {
            let members: Vec<_> = archive
                .features
                .features
                .iter()
                .filter(|f| &f.group == g && f.status == MeasurementStatus::Observed)
                .collect();
            Attribution {
                group: g.clone(),
                feature_ids: members.iter().map(|f| f.id.clone()).collect(),
                label: if members.len() == 1 {
                    members[0].name.clone()
                } else {
                    format!("{g} ({} features)", members.len())
                },
                modalities: members
                    .iter()
                    .map(|f| f.modality)
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect(),
                contribution: values[i],
                sampling_standard_error: errors[i],
            }
        })
        .collect();
    Ok(ExplanationResult { method:format!("{} / {} / grouped model-input probability Shapley",match archive.config.baseline {Baseline::MaskedEvidence=>"masked-evidence",Baseline::ReferenceBackground=>"reference-background"},match archive.config.algorithm {Algorithm::Exact=>"exact",Algorithm::Permutation=>"permutation"}),target_class:archive.config.target_class.clone(),baseline_probability:baseline,full_probability:full,attributions,additivity_residual:residual,evaluations:archive.evaluations.len(),input_tokens:archive.tokens(),background_sha256:archive.background.as_ref().map(molecular::hash).transpose()?,limitations:vec!["Raw model-output attribution, not clinical probability or causality.".into(),"Features in a dependency/compositional group are explained jointly; no per-member attribution is invented.".into(),"Sampling error excludes provider variation and background-selection uncertainty.".into(),"Archived responses are reproducible but self-reported; hashes do not authenticate provider origin.".into()] })
}
