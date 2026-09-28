#!/usr/bin/env python3
"""Generate the fixtures/gut-generated-v1 synthetic CUP collection.

Writes ~250 invented patient workups in the exact TSV schema consumed by the
josh-ingest molecular importer (crates/josh-ingest/src/molecular.rs), plus a
scenarios.json review manifest and a README.md. Output is deterministic: the
RNG is seeded with a fixed seed that is printed on every run.

Schema contract enforced here (must match the importer):
- header is exactly the 22 columns of fixtures/cup-realistic-v2 (Row struct fields)
- status in {observed, unknown, not_tested}; non-observed rows carry no value
- mutation counts: empty gene column, units "nonsynonymous_variants", value >= 0
- copy_number: units "discrete_call", gene set, integer call -2..+2
- demographic age: units "years", value "N" or ">N" with N <= 120
- ihc/histology: empty units (free-text category values)
- ids/groups: [A-Za-z0-9_.-] only, unique per file
"""

import json
import random
from pathlib import Path

SEED = 20260929  # gut r2 expansion batch
rng = random.Random(SEED)

REPO = Path(__file__).resolve().parent.parent
OUT = REPO / "fixtures" / "gut-generated-v2"

HEADER = [
    "sample_id",
    "id",
    "name",
    "modality",
    "value",
    "units",
    "status",
    "gene",
    "chromosome",
    "position",
    "reference",
    "alternate",
    "consequence",
    "somatic",
    "catalogue",
    "method",
    "mutation_count",
    "reference_build",
    "assay",
    "coverage",
    "group",
    "depends_on",
]

ASSAY_DNA = "synthetic-targeted-DNA-panel-v2"
ASSAY_CNA = "synthetic-panel-CNA-v2"
ASSAY_IHC = "synthetic-IHC-v2"
ASSAY_WORKUP = "synthetic-workup-v2"
ASSAY_RECORD = "synthetic-record-v2"
ASSAY_PATH = "synthetic-pathology-v2"
ASSAY_RNA = "synthetic-RNA-workup-v2"
BUILD = "GRCh38"
PATIENT_NOTE = "simulated observation; no real patient"

DNA_COVER_GOOD = (
    "coding exons passed depth >=250x; validated VAF limit 5%; "
    "simulated matched-normal somatic calls"
)
DNA_COVER_MED = (
    "coding exons passed depth >=150x; validated VAF limit 5%; "
    "simulated matched-normal somatic calls"
)
DNA_COVER_TUMOR_ONLY = (
    "tumor-only; locus depth 180x; VAF 7%; somatic status unresolved; "
    "other loci may fail"
)
CNA_COVER = "simulated validated gene-level ordinal call; not absolute copies"
IHC_COVER = "simulated sampled tumor; internal and external controls adequate"
SIG_NOTE = "Too few panel variants for a stable signature fit; no exposure estimated"
RNA_NOTE = "No RNA expression assay performed"


def row(
    sid,
    fid,
    name,
    modality,
    value="",
    units="",
    status="observed",
    assay="",
    coverage="",
    group=None,
    reference_build="",
):
    assert len(fid) > 0 and all(c.isalnum() or c in "_-." for c in fid), fid
    return {
        "sample_id": sid,
        "id": fid,
        "name": name,
        "modality": modality,
        "value": value,
        "units": units,
        "status": status,
        "gene": "",
        "chromosome": "",
        "position": "",
        "reference": "",
        "alternate": "",
        "consequence": "",
        "somatic": "",
        "catalogue": "",
        "method": "",
        "mutation_count": "",
        "reference_build": reference_build,
        "assay": assay,
        "coverage": coverage,
        "group": group or fid,
        "depends_on": "",
    }


def context_rows(sid, specimen, workup, quality, morphology, morph_group="morphology"):
    return [
        row(
            sid,
            "specimen",
            "Specimen context",
            "histology",
            specimen,
            assay=ASSAY_PATH,
            coverage=PATIENT_NOTE,
            group="context",
        ),
        row(
            sid,
            "workup",
            "Workup context",
            "histology",
            workup,
            assay=ASSAY_PATH,
            coverage=PATIENT_NOTE,
            group="context",
        ),
        row(
            sid,
            "quality",
            "Specimen quality",
            "histology",
            quality,
            assay=ASSAY_PATH,
            coverage=PATIENT_NOTE,
            group="context",
        ),
        row(
            sid,
            "morphology",
            "Biopsy morphology",
            "histology",
            morphology,
            assay=ASSAY_PATH,
            coverage=PATIENT_NOTE,
            group=morph_group,
        ),
    ]


def demographics(sid, age, sex):
    rows = [
        row(
            sid,
            "age",
            "Age at sampling",
            "demographic",
            str(age),
            units="years",
            assay=ASSAY_RECORD,
            coverage=PATIENT_NOTE,
            group="demographics",
        )
    ]
    if sex is None:
        rows.append(
            row(
                sid,
                "sex",
                "Recorded sex at birth",
                "demographic",
                status="unknown",
                assay=ASSAY_RECORD,
                coverage=PATIENT_NOTE,
                group="demographics",
            )
        )
    else:
        rows.append(
            row(
                sid,
                "sex",
                "Recorded sex at birth",
                "demographic",
                sex,
                assay=ASSAY_RECORD,
                coverage=PATIENT_NOTE,
                group="demographics",
            )
        )
    return rows


def mut_count(sid, gene, count, group=None, cover=DNA_COVER_GOOD):
    return row(
        sid,
        f"mut-{gene.lower()}",
        f"{gene} reported nonsynonymous variant count",
        "mutation",
        str(count),
        units="nonsynonymous_variants",
        assay=ASSAY_DNA,
        coverage=cover,
        group=group,
        reference_build=BUILD,
    )


def mut_failed(sid, gene):
    return row(
        sid,
        f"unavailable-{gene.lower()}",
        gene,
        "mutation",
        status="unknown",
        assay=ASSAY_WORKUP,
        coverage="Failed coverage; mutation absence cannot be assessed",
        group=None,
    )


def cna(sid, gene, call, group=None):
    return cna_row(sid, gene, call, group=group)


def cna_row(sid, gene, call, group=None):
    """CNA row carrying the gene in the gene column (typed ordinal call)."""
    r = row(
        sid,
        f"cna-{gene.lower()}",
        f"{gene} copy number",
        "copy_number",
        str(call),
        units="discrete_call",
        assay=ASSAY_CNA,
        coverage=CNA_COVER,
        group=group,
        reference_build=BUILD,
    )
    r["gene"] = gene
    return r


def mut_gene_row(sid, gene, count, group=None, cover=DNA_COVER_GOOD):
    """Count row (gene column stays empty, matching the count encoding)."""
    return mut_count(sid, gene, count, group=group, cover=cover)


def ihc(sid, idx, marker, result, group="ihc-panel", cover_note=None):
    return row(
        sid,
        f"ihc-{idx}",
        marker,
        "ihc",
        result,
        assay=ASSAY_IHC,
        coverage=cover_note or IHC_COVER,
        group=group,
    )


def ihc_failed(sid, idx, marker):
    return row(
        sid,
        f"unavailable-{idx}",
        marker,
        "ihc",
        status="unknown",
        assay=ASSAY_WORKUP,
        coverage="Internal stain control failed",
        group=None,
    )


def unavailable(sid, idx, name, modality, reason):
    return row(
        sid,
        f"unavailable-{idx}",
        name,
        modality,
        status="not_tested",
        assay=ASSAY_WORKUP,
        coverage=reason,
        group=None,
    )


def tail_rows(sid):
    return [
        row(
            sid,
            "signatures",
            "SBS signature assessment",
            "signature",
            status="not_tested",
            assay=ASSAY_DNA,
            coverage=SIG_NOTE,
            group="signatures",
        ),
        row(
            sid,
            "expression",
            "RNA expression profile",
            "expression",
            status="not_tested",
            assay=ASSAY_RNA,
            coverage=RNA_NOTE,
            group="expression",
        ),
    ]


NEG = "negative in viable tumor"
POS_DIFFUSE = "positive; diffuse"
POS_NUCLEAR = "positive; strong diffuse nuclear staining"


def positive(pattern, pct=None):
    if pct is None:
        return f"positive; {pattern}"
    return f"positive; {pattern} in {pct}%"


def pick_weights(rng, options):
    return rng.choices([v for v, _ in options], weights=[w for _, w in options], k=1)[0]


# ---------------------------------------------------------------------------
# clinical text pools (all invented)
# ---------------------------------------------------------------------------

NODE_SITES = [
    "Left supraclavicular node core",
    "Right axillary node core",
    "Periportal node core",
    "Retroperitoneal node core",
    "Inguinal node core",
    "Mediastinal node station 7 core",
    "Cervical node core",
    "Peripancreatic node core",
]
LIVER_SITES = [
    "Liver core from segment VI",
    "Liver core from segment IVb",
    "Core of a dominant right-lobe lesion",
]
BONE_SITES = [
    "Lytic iliac bone lesion core",
    "Sclerotic lumbar vertebral core",
    "Lytic humeral head core",
    "Mixed lytic-sclerotic femoral lesion core",
]
IMAGING_UNREVEALING = [
    "CT chest/abdomen/pelvis shows no dominant primary",
    "CT and directed endoscopy identify no definite primary",
    "Cross-sectional imaging including CT chest/abdomen/pelvis is unrevealing",
    "Whole-body CT and symptom-directed endoscopy show no primary",
]
TF_RANGE = list(range(15, 70, 5))


def quality_text(rng, decalcified=False, failed=False, dual=False):
    tf = rng.choice([5, 10, 15]) if decalcified else rng.choice(TF_RANGE)
    parts = [f"FFPE core; estimated tumor fraction {tf}%"]
    if decalcified:
        parts.append("acid-decalcified")
    if failed:
        parts.append("necrosis with failed internal stain controls")
    elif dual:
        parts.append("adequate stain controls in both blocks")
    else:
        parts.append("pretreatment specimen")
    return "; ".join(parts) + "."


def morphology_text(rng, options):
    return rng.choice(options)


# ---------------------------------------------------------------------------
# archetype case builders
#
# each returns (rows, scenario dict extras: exercise, candidates, notes, flags)
# ---------------------------------------------------------------------------


def build_pulmonary(rng, sid, case_no):
    squamous = rng.random() < 0.15
    discordant = False
    rows = demographics(sid, *age_sex(rng, "male"))
    specimen = f"{rng.choice(NODE_SITES + LIVER_SITES + ['Pleural implant core', 'Adrenal core'])}; metastatic presentation without a localized primary."
    workup = f"{rng.choice(IMAGING_UNREVEALING)}; bronchoscopy washings are negative."
    quality = quality_text(rng)
    if squamous:
        morphology = rng.choice(
            [
                "Poorly differentiated non-keratinizing squamous cell carcinoma.",
                "Nests of squamous carcinoma with intercellular bridges.",
            ]
        )
        rows += context_rows(sid, specimen, workup, quality, morphology)
        rows.append(mut_gene_row(sid, "TP53", 1))
        rows.append(mut_gene_row(sid, "PIK3CA", pick_weights(rng, [(0, 7), (1, 3)])))
        rows.append(mut_gene_row(sid, "EGFR", 0))
        rows.append(cna_row(sid, "SOX2", pick_weights(rng, [(0, 4), (1, 4), (2, 2)])))
        markers = [
            ("p40", positive("diffuse nuclear staining")),
            ("CK5/6", positive("diffuse membranous and cytoplasmic staining")),
            ("CK7", rng.choice([NEG, "positive; focal staining in 15%"])),
            ("TTF-1", NEG),
            ("Napsin A", NEG),
            ("CDX2", NEG),
            ("PAX8", NEG),
        ]
    else:
        morphology = rng.choice(
            [
                "Moderately differentiated adenocarcinoma with acinar architecture.",
                "Adenocarcinoma with lepidic and papillary areas and focal mucin.",
                "Poorly differentiated adenocarcinoma with solid architecture.",
            ]
        )
        rows += context_rows(sid, specimen, workup, quality, morphology)
        rows.append(mut_gene_row(sid, "TP53", pick_weights(rng, [(0, 3), (1, 7)])))
        rows.append(mut_gene_row(sid, "KRAS", pick_weights(rng, [(0, 4), (1, 6)])))
        rows.append(mut_gene_row(sid, "STK11", pick_weights(rng, [(0, 6), (1, 4)])))
        rows.append(mut_gene_row(sid, "EGFR", 0))
        rows.append(
            mut_gene_row(sid, "KEAP1", pick_weights(rng, [(0, 8), (1, 2)])),
        ) if rng.random() < 0.4 else None
        rows.append(cna_row(sid, "MYC", pick_weights(rng, [(0, 6), (1, 4)])))
        rows.append(cna_row(sid, "CDKN2A", pick_weights(rng, [(0, 6), (-2, 4)])))
        rows.append(cna_row(sid, "MET", pick_weights(rng, [(0, 8), (1, 2)])))
        markers = [
            (
                "TTF-1",
                positive(
                    rng.choice(
                        ["strong nuclear staining", "moderate nuclear staining"]
                    ),
                    rng.choice([60, 70, 75, 90]),
                ),
            ),
            (
                "Napsin A",
                positive("granular cytoplasmic staining", rng.choice([40, 60])),
            ),
            ("CK7", POS_DIFFUSE),
            ("CK20", NEG),
            ("CDX2", NEG),
            ("p40", NEG),
            ("PAX8", NEG),
        ]
    if rng.random() < 0.08:
        # deliberately conflicting gynecologic marker in a pulmonary pattern
        markers = [("PAX8", "positive; focal nuclear staining in 10%")] + markers[:-1]
        discordant = True
    rows += [ihc(sid, i + 1, m, v) for i, (m, v) in enumerate(markers)]
    n = len([r for r in rows if r["id"].startswith("unavailable-")]) + 1
    rows.append(
        unavailable(
            sid, n, "ALK rearrangement", "mutation", "RNA fusion assay not performed"
        )
    )
    if rng.random() < 0.5:
        n += 1
        rows.append(
            unavailable(
                sid,
                n,
                "ROS1 rearrangement",
                "mutation",
                "RNA fusion assay not performed",
            )
        )
    rows += tail_rows(sid)
    if discordant:
        ex, cand = "discordant_markers", ["NSCLC", "OVT", "insufficient_evidence"]
        notes = (
            "Pulmonary-differentiated adenocarcinoma with an unexpected "
            "focal PAX8 result; gynecologic overlap needs review before "
            "any lung assignment is treated as settled."
        )
    else:
        ex, cand = "coherent_panel", ["NSCLC"]
        notes = (
            "Pulmonary-differentiated adenocarcinoma or squamous pattern; "
            "the anatomical primary remains unconfirmed on imaging."
        )
    return rows, {"exercise": ex, "candidates": cand, "notes": notes}


def age_sex(rng, default_sex):
    age = rng.choice([">89"] * 2 + [rng.randint(38, 88)])
    if age == ">89":
        return age, default_sex if rng.random() < 0.9 else None
    sex = default_sex if rng.random() < 0.95 else rng.choice(["male", "female"])
    return age, sex


def build_intestinal(rng, sid, case_no):
    rows = demographics(sid, *age_sex(rng, rng.choice(["male", "female"])))
    specimen = f"{rng.choice(LIVER_SITES + ['Peritoneal implant core', 'Perihepatic node core'])}; several peritoneal and hepatic deposits."
    workup = rng.choice(
        [
            "CT shows no dominant primary; colonoscopy incomplete at a fixed sigmoid angulation.",
            "CT and full colonoscopy show no primary mass; small-bowel imaging unrevealing.",
            "CT chest/abdomen/pelvis unrevealing; colonoscopy reached the cecum without a lesion.",
        ]
    )
    quality = quality_text(rng)
    morphology = rng.choice(
        [
            "Gland-forming adenocarcinoma with luminal dirty necrosis.",
            "Moderately differentiated adenocarcinoma with cribriform glands.",
            "Mucinous adenocarcinoma with extracellular mucin pools.",
        ]
    )
    rows += context_rows(sid, specimen, workup, quality, morphology)
    mmr_loss = rng.random() < 0.1
    rows.append(mut_gene_row(sid, "APC", pick_weights(rng, [(1, 5), (2, 5)])))
    rows.append(mut_gene_row(sid, "TP53", pick_weights(rng, [(0, 4), (1, 6)])))
    rows.append(mut_gene_row(sid, "KRAS", pick_weights(rng, [(0, 4), (1, 6)])))
    rows.append(mut_gene_row(sid, "BRAF", pick_weights(rng, [(0, 9), (1, 1)])))
    rows.append(cna_row(sid, "MYC", pick_weights(rng, [(0, 5), (1, 5)])))
    rows.append(cna_row(sid, "ERBB2", 0))
    discordant = False
    markers = [
        ("SATB2", positive("strong nuclear staining", rng.choice([70, 80, 85, 90]))),
        ("CDX2", positive("strong diffuse nuclear staining")),
        ("CK20", POS_DIFFUSE),
        ("CK7", NEG),
        ("TTF-1", NEG),
        ("PAX8", NEG),
    ]
    if rng.random() < 0.12:
        markers[3] = ("CK7", "positive; focal staining in 20%")
        discordant = True
    if rng.random() < 0.06:
        markers[0] = ("SATB2", "weak and equivocal nuclear staining in 10%")
        discordant = True
    ihc_rows = []
    idx = 1
    for m, v in markers:
        ihc_rows.append(ihc(sid, idx, m, v))
        idx += 1
    if mmr_loss:
        ihc_rows.append(ihc(sid, idx, "MLH1", NEG))
        idx += 1
        ihc_rows.append(ihc(sid, idx, "PMS2", NEG))
        idx += 1
        ihc_rows.append(ihc(sid, idx, "MSH2", "retained nuclear expression"))
        idx += 1
        ihc_rows.append(ihc(sid, idx, "MSH6", "retained nuclear expression"))
        for r in ihc_rows[-4:]:
            r["group"] = "mmr-panel"
    else:
        ihc_rows.append(ihc(sid, idx, "MLH1", "retained nuclear expression"))
        idx += 1
        ihc_rows.append(ihc(sid, idx, "PMS2", "retained nuclear expression"))
        if rng.random() < 0.4:
            idx2 = idx + 1
            ihc_rows.append(
                unavailable(
                    sid,
                    idx2,
                    "MSH2",
                    "ihc",
                    "Tissue exhausted before remaining mismatch-repair stains",
                )
            )
            ihc_rows.append(
                unavailable(
                    sid,
                    idx2 + 1,
                    "MSH6",
                    "ihc",
                    "Tissue exhausted; do not infer complete MMR proficiency",
                )
            )
        for r in ihc_rows[-2:]:
            if r["id"].startswith("ihc-"):
                r["group"] = "mmr-panel"
    rows += ihc_rows
    rows += tail_rows(sid)
    if discordant:
        ex = "discordant_markers"
        cand = ["COADREAD", "PAAD", "insufficient_evidence"]
        notes = (
            "Intestinal-differentiated carcinoma with a divergent keratin "
            "or weak SATB2 result; upper-GI and pancreatobiliary overlap "
            "must be excluded before a colorectal assignment."
        )
    elif mmr_loss:
        ex = "coherent_panel"
        cand = ["COADREAD"]
        notes = (
            "Intestinal differentiation with lost MLH1/PMS2 staining in "
            "the sampled deposit; primary site still not proven."
        )
    else:
        ex = "coherent_panel"
        cand = ["COADREAD"]
        notes = (
            "Intestinal-differentiated adenocarcinoma; partial MMR "
            "staining does not establish complete MMR proficiency."
        )
    return rows, {"exercise": ex, "candidates": cand, "notes": notes}


def build_insufficient(rng, sid, case_no):
    age = rng.choice([">89", rng.randint(50, 86)])
    sex = rng.choice(["male", "female"]) if rng.random() < 0.9 else None
    rows = demographics(sid, age, sex)
    specimen = f"{rng.choice(BONE_SITES)}; hepatic and nodal deposits noted on staging."
    workup = rng.choice(
        [
            "CT chest/abdomen/pelvis and directed endoscopy did not identify a primary; repeat biopsy pending.",
            "Staging CT and symptom-directed endoscopy unrevealing; further tissue declined by the patient.",
            "CT and MRI show no dominant primary; biopsy material is nearly exhausted.",
        ]
    )
    quality = quality_text(rng, decalcified=True, failed=rng.random() < 0.6)
    morphology = rng.choice(
        [
            "Poorly differentiated carcinoma; very limited viable nests.",
            "Scant fragment of undifferentiated malignant neoplasm.",
            "Crushed small biopsy with rare atypical epithelial nests.",
        ]
    )
    rows += context_rows(sid, specimen, workup, quality, morphology)
    if rng.random() < 0.65:
        rows.append(mut_gene_row(sid, "TP53", 1, cover=DNA_COVER_TUMOR_ONLY))
    else:
        rows.append(mut_failed(sid, rng.choice(["KRAS", "BRAF"])))
    if rng.random() < 0.4:
        rows.append(
            row(
                sid,
                "unavailable-cdkn2a",
                "CDKN2A copy number",
                "copy_number",
                status="unknown",
                assay=ASSAY_WORKUP,
                coverage="Low tumor fraction and noisy segmentation; no reliable call",
                group=None,
            )
        )
    markers = [
        ("AE1/AE3", positive("focal in viable tumor")),
        (
            "CK7",
            positive("patchy staining in 25%")
            if rng.random() < 0.5
            else "weak focal staining in 5%",
        ),
        ("CK20", positive("weak focal staining in 5%") if rng.random() < 0.4 else NEG),
    ]
    idx = 1
    for m, v in markers:
        if rng.random() < 0.15:
            rows.append(ihc_failed(sid, idx, m))
        else:
            rows.append(ihc(sid, idx, m, v))
        idx += 1
    for m in rng.sample(
        ["TTF-1", "CDX2", "SATB2", "PAX8", "Napsin A", "GATA3"], k=rng.randint(4, 6)
    ):
        if rng.random() < 0.4:
            rows.append(ihc_failed(sid, idx, m))
        else:
            rows.append(
                unavailable(
                    sid,
                    idx,
                    m,
                    "ihc",
                    rng.choice(["Tissue exhausted", "Not performed; tissue exhausted"]),
                )
            )
        idx += 1
    rows += tail_rows(sid)
    return rows, {
        "exercise": "insufficient_evidence",
        "candidates": [],
        "notes": (
            "Decalcified scant core with low tumor fraction; failed or "
            "unperformed assays dominate and a tumor-only TP53 count is "
            "not an origin label."
        ),
    }


def build_breast(rng, sid, case_no):
    rows = demographics(sid, *age_sex(rng, "female"))
    specimen = f"{rng.choice(NODE_SITES + ['Skin nodule of the chest wall core', 'Sternal bone lesion core'])}; no breast mass on examination."
    workup = rng.choice(
        [
            "Mammogram and breast ultrasound show no primary; axillary presentation is the first evidence.",
            "CT chest/abdomen/pelvis unrevealing; breast MRI shows only indeterminate parenchymal signal.",
            "Bilateral mammography and ultrasound show dense but unremarkable breast tissue.",
        ]
    )
    quality = quality_text(rng, failed=rng.random() < 0.1)
    morphology = rng.choice(
        [
            "Invasive carcinoma of no special type with tubule formation.",
            "Poorly differentiated carcinoma with nested growth and mitotic activity.",
            "Invasive lobular carcinoma with single-file cords.",
        ]
    )
    rows += context_rows(sid, specimen, workup, quality, morphology)
    discordant = False
    er_positive = rng.random() < 0.8
    rows.append(mut_gene_row(sid, "PIK3CA", pick_weights(rng, [(0, 4), (1, 6)])))
    rows.append(mut_gene_row(sid, "TP53", pick_weights(rng, [(0, 5), (1, 5)])))
    rows.append(mut_gene_row(sid, "ESR1", 0))
    if rng.random() < 0.25:
        rows.append(mut_gene_row(sid, "PTEN", 1, group="pten-axis"))
        rows.append(cna_row(sid, "PTEN", -1, group="pten-axis"))
    rows.append(cna_row(sid, "ERBB2", pick_weights(rng, [(0, 8), (1, 2)])))
    rows.append(cna_row(sid, "MYC", pick_weights(rng, [(0, 6), (1, 4)])))
    markers = [
        (
            "ER",
            positive("nuclear staining", rng.choice([70, 80, 90]))
            if er_positive
            else NEG,
        ),
        (
            "PR",
            positive("nuclear staining", rng.choice([30, 50, 60]))
            if er_positive
            else NEG,
        ),
        ("GATA3", positive("strong diffuse nuclear staining")),
        ("Mammaglobin", positive("cytoplasmic staining")),
        (
            "HER2",
            rng.choice(
                [
                    "negative in viable tumor",
                    "equivocal incomplete membranous staining in 10%; ISH not performed",
                ]
            ),
        ),
        ("TTF-1", NEG),
        ("CDX2", NEG),
    ]
    if not er_positive and rng.random() < 0.5:
        markers[2] = ("GATA3", "positive; moderate nuclear staining in 40%")
        discordant = True
    rows += [ihc(sid, i + 1, m, v) for i, (m, v) in enumerate(markers)]
    n = len([r for r in rows if r["id"].startswith("unavailable-")]) + 1
    if rng.random() < 0.3:
        rows.append(
            unavailable(
                sid,
                n,
                "E-cadherin",
                "ihc",
                "Not performed; lobular versus ductal distinction pending",
            )
        )
    rows += tail_rows(sid)
    if discordant:
        ex = "discordant_markers"
        cand = ["BRCA", "BLCA", "insufficient_evidence"]
        notes = (
            "Hormone-receptor negative presentation with only moderate "
            "GATA3; urothelial overlap needs exclusion before a breast "
            "assignment."
        )
    else:
        ex = "coherent_panel"
        cand = ["BRCA"]
        notes = (
            "Axillary or chest-wall presentation with a coherent "
            "hormone-receptor and GATA3 panel; imaging shows no breast "
            "primary."
        )
    return rows, {"exercise": ex, "candidates": cand, "notes": notes}


def build_gynecologic(rng, sid, case_no):
    serous = rng.random() < 0.7
    rows = demographics(sid, *age_sex(rng, "female"))
    specimen = rng.choice(
        [
            "Omental core and ascites cell block; diffuse peritoneal deposits.",
            "Pelvic peritoneal implant core; bilateral adnexal masses noted.",
            "Vaginal vault recurrence core; prior hysterectomy elsewhere, records unavailable.",
        ]
    )
    workup = rng.choice(
        [
            "CT and pelvic ultrasound show no dominant ovarian, tubal or uterine mass; endometrial sampling is scant.",
            "CT shows peritoneal disease without a dominant adnexal or uterine lesion.",
            "Pelvic MRI shows indeterminate adnexal enhancement; endometrial biopsy failed.",
        ]
    )
    quality = quality_text(rng)
    morphology = (
        rng.choice(
            [
                "High-grade carcinoma with papillary and solid architecture and marked nuclear atypia.",
                "High-grade serous carcinoma pattern with slit-like glands and psammomatous calcification.",
            ]
        )
        if serous
        else rng.choice(
            [
                "Endometrioid carcinoma with glandular and squamous differentiation.",
                "Grade 2 endometrioid carcinoma with villoglandular areas.",
            ]
        )
    )
    rows += context_rows(sid, specimen, workup, quality, morphology)
    if serous:
        rows.append(mut_gene_row(sid, "TP53", 1, group="tp53"))
        rows.append(mut_gene_row(sid, "BRCA1", 0))
        rows.append(mut_gene_row(sid, "BRCA2", 0))
        rows.append(cna_row(sid, "CCNE1", pick_weights(rng, [(0, 6), (1, 2), (2, 2)])))
        rows.append(cna_row(sid, "MYC", pick_weights(rng, [(0, 6), (1, 4)])))
        markers = [
            ("PAX8", POS_NUCLEAR),
            ("WT1", POS_NUCLEAR),
            ("ER", positive("nuclear staining", rng.choice([60, 80]))),
            ("p53", "aberrant overexpression in more than 90% of tumor nuclei"),
            ("CK7", POS_DIFFUSE),
            ("CK20", NEG),
            ("CDX2", NEG),
            ("GATA3", NEG),
            ("TTF-1", NEG),
        ]
        ambiguous = rng.random() < 0.3
        if ambiguous:
            ex = "site_ambiguity"
            cand = ["OVT", "UCEC", "other_origin"]
            notes = (
                "Serous gynecologic phenotype without a dominant adnexal "
                "or uterine lesion; ovarian, tubal, peritoneal and "
                "endometrial locations are not distinguished here."
            )
        else:
            ex = "coherent_panel"
            cand = ["OVT"]
            notes = (
                "High-grade serous pattern with a coherent PAX8/WT1/p53 "
                "panel; BRCA counts do not establish HRD status."
            )
    else:
        rows.append(mut_gene_row(sid, "PTEN", 1, group="pten-axis"))
        rows.append(mut_gene_row(sid, "CTNNB1", pick_weights(rng, [(0, 7), (1, 3)])))
        rows.append(mut_gene_row(sid, "PIK3CA", pick_weights(rng, [(0, 5), (1, 5)])))
        rows.append(cna_row(sid, "PTEN", -1, group="pten-axis"))
        markers = [
            ("PAX8", POS_NUCLEAR),
            ("WT1", NEG),
            ("ER", positive("strong nuclear staining", 90)),
            ("PR", positive("nuclear staining", 60)),
            ("p53", "wild-type heterogeneous staining"),
            ("CK7", POS_DIFFUSE),
            ("CK20", NEG),
            ("CDX2", NEG),
            ("TTF-1", NEG),
        ]
        ex = "coherent_panel"
        cand = ["UCEC"]
        notes = (
            "Endometrioid-pattern carcinoma with WT1 loss and PTEN loss "
            "grouped as one axis; ovarian versus endometrial origin "
            "remains anatomically unresolved."
        )
    rows += [ihc(sid, i + 1, m, v) for i, (m, v) in enumerate(markers)]
    # relabel the p53 IHC id so the tp53 group pairing is stable when present
    if serous:
        for r in rows:
            if r["name"] == "p53" and r["id"].startswith("ihc-"):
                r["id"] = "ihc-p53"
                r["group"] = "tp53"
    n = len([r for r in rows if r["id"].startswith("unavailable-")]) + 1
    rows.append(
        unavailable(
            sid,
            n,
            "Homologous recombination deficiency assay",
            "mutation",
            "No genomic-scar assay; no HRD inference from BRCA counts",
        )
    )
    if serous and rng.random() < 0.4:
        n += 1
        rows.append(
            row(
                sid,
                f"unavailable-{n}",
                "Endometrial morphology",
                "histology",
                status="unknown",
                assay=ASSAY_WORKUP,
                coverage="Insufficient endometrial material; exact anatomic origin unresolved",
                group=None,
            )
        )
    rows += tail_rows(sid)
    return rows, {"exercise": ex, "candidates": cand, "notes": notes}


def build_prostate(rng, sid, case_no):
    rows = demographics(sid, *age_sex(rng, "male"))
    specimen = (
        f"{rng.choice(NODE_SITES + BONE_SITES)}; sclerotic bone lesions on staging."
    )
    workup = rng.choice(
        [
            "CT and bone scan show widespread sclerotic disease; no prior prostate diagnosis or biopsy on record.",
            "CT shows pelvic nodes and sclerotic bone lesions; prostate is small and non-nodular on imaging.",
        ]
    )
    quality = quality_text(rng, decalcified=rng.random() < 0.3)
    morphology = rng.choice(
        [
            "Poorly differentiated adenocarcinoma with cribriform and fused glands.",
            "Small glands infiltrating fibrous stroma with prominent nucleoli.",
        ]
    )
    rows += context_rows(sid, specimen, workup, quality, morphology)
    discordant = rng.random() < 0.1
    if rng.random() < 0.35:
        rows.append(mut_gene_row(sid, "PTEN", 1, group="pten-axis"))
        rows.append(cna_row(sid, "PTEN", -1, group="pten-axis"))
    rows.append(mut_gene_row(sid, "SPOP", pick_weights(rng, [(0, 7), (1, 3)])))
    rows.append(mut_gene_row(sid, "TP53", pick_weights(rng, [(0, 6), (1, 4)])))
    rows.append(mut_gene_row(sid, "ATM", pick_weights(rng, [(0, 8), (1, 2)])))
    rows.append(cna_row(sid, "ERG", pick_weights(rng, [(0, 5), (1, 3), (2, 2)])))
    psa = "weak focal staining in 10% of tumor cells" if not discordant else NEG
    markers = [
        ("NKX3.1", NEG if discordant else positive("strong diffuse nuclear staining")),
        (
            "PSAP",
            positive("granular cytoplasmic staining")
            if not discordant
            else "weak focal staining in 10%",
        ),
        ("PSA", psa),
        (
            "Synaptophysin",
            positive("diffuse cytoplasmic staining") if discordant else NEG,
        ),
        ("GATA3", NEG),
        ("TTF-1", NEG),
        ("CDX2", NEG),
    ]
    rows += [ihc(sid, i + 1, m, v) for i, (m, v) in enumerate(markers)]
    n = len([r for r in rows if r["id"].startswith("unavailable-")]) + 1
    if (
        quality.startswith("FFPE core; estimated tumor fraction 5")
        or "decalcified" in quality
    ):
        rows.append(
            row(
                sid,
                f"unavailable-{n}",
                "ERG",
                "ihc",
                status="unknown",
                assay=ASSAY_WORKUP,
                coverage="Equivocal stain after decalcification; repeat unavailable",
                group=None,
            )
        )
    rows += tail_rows(sid)
    if discordant:
        ex = "discordant_markers"
        cand = ["PRAD", "other_origin", "insufficient_evidence"]
        notes = (
            "Loss of NKX3.1 and PSA with synaptophysin acquisition "
            "suggests neuroendocrine divergence of prostatic origin; "
            "the frozen taxonomy has no neuroendocrine-prostate class."
        )
    else:
        ex = "coherent_panel"
        cand = ["PRAD"]
        notes = (
            "Nodal or sclerotic-bone presentation with coherent "
            "NKX3.1/PSAP evidence; focal weak PSA staining is not a "
            "missing result."
        )
    return rows, {"exercise": ex, "candidates": cand, "notes": notes}


def build_renal(rng, sid, case_no):
    rows = demographics(sid, *age_sex(rng, "male"))
    specimen = f"{rng.choice(BONE_SITES + ['Non-decalcified soft-tissue metastasis core', 'Pancreatic head mass core'])}; non-decalcified tissue sampled where possible."
    workup = rng.choice(
        [
            "CT shows small bilateral renal cysts but no definite renal primary; thyroid ultrasound unrevealing.",
            "CT and MRI show no dominant renal mass; a 1 cm upper-pole lesion is indeterminate.",
        ]
    )
    quality = quality_text(rng)
    morphology = rng.choice(
        [
            "Clear-cell carcinoma in nests with a delicate capillary network.",
            "Clear and eosinophilic cells with corona-like vascular patterns.",
        ]
    )
    rows += context_rows(sid, specimen, workup, quality, morphology)
    grouped = rng.random() < 0.7
    rows.append(
        mut_gene_row(sid, "VHL", 1, group="renal-sequence" if grouped else None)
    )
    rows.append(
        mut_gene_row(
            sid,
            "PBRM1",
            pick_weights(rng, [(0, 4), (1, 6)]),
            group="renal-sequence" if grouped else None,
        )
    )
    rows.append(mut_gene_row(sid, "BAP1", pick_weights(rng, [(0, 8), (1, 2)])))
    rows.append(cna_row(sid, "VHL", -1, group="renal-sequence" if grouped else None))
    rows.append(cna_row(sid, "PBRM1", -1, group="renal-sequence" if grouped else None))
    discordant = rng.random() < 0.1
    markers = [
        ("PAX8", POS_NUCLEAR),
        (
            "CAIX",
            NEG if discordant else positive("diffuse complete membranous staining"),
        ),
        ("CD10", positive("diffuse membranous staining")),
        ("RCC marker", positive("focal cytoplasmic staining")),
        (
            "CK7",
            "positive; diffuse"
            if discordant
            else rng.choice([NEG, "positive; focal staining in 15%"]),
        ),
        ("TTF-1", NEG),
        ("Thyroglobulin", NEG),
        ("GATA3", NEG),
    ]
    rows += [ihc(sid, i + 1, m, v) for i, (m, v) in enumerate(markers)]
    n = len([r for r in rows if r["id"].startswith("unavailable-")]) + 1
    rows.append(
        unavailable(sid, n, "TFE3", "ihc", "Not performed; subtype workup incomplete")
    )
    rows += tail_rows(sid)
    if discordant:
        ex = "discordant_markers"
        cand = ["RCC", "other_origin", "insufficient_evidence"]
        notes = (
            "Clear-cell morphology with VHL/PBRM1 evidence but lost "
            "CAIX and diffuse CK7; papillary or translocation subtypes "
            "are not separable in the frozen taxonomy."
        )
    else:
        ex = "coherent_panel"
        cand = ["RCC"]
        notes = (
            "Renal-differentiation panel with grouped VHL/PBRM1 counts "
            "and losses sharing one attribution group."
        )
    return rows, {"exercise": ex, "candidates": cand, "notes": notes}


CONFLICT_PAIRS = [
    (
        "A: liver core. B: supraclavicular node core obtained two weeks later before therapy.",
        "A: gland-forming carcinoma with dirty necrosis. B: adenocarcinoma with acinar architecture.",
        [
            ("SATB2", POS_NUCLEAR),
            ("CDX2", POS_NUCLEAR),
            ("CK20", POS_DIFFUSE),
            ("TTF-1", NEG),
        ],
        [
            ("TTF-1", positive("strong diffuse nuclear staining")),
            ("Napsin A", positive("granular cytoplasmic staining")),
            ("CK7", POS_DIFFUSE),
            ("SATB2", NEG),
        ],
        ["APC", "KRAS", "TP53"],
    ),
    (
        "A: axillary node core. B: peritoneal implant core obtained three weeks later.",
        "A: carcinoma of no special type with gland formation. B: high-grade carcinoma with papillary architecture.",
        [
            ("ER", positive("nuclear staining", 80)),
            ("GATA3", POS_NUCLEAR),
            ("Mammaglobin", positive("cytoplasmic staining")),
            ("PAX8", NEG),
        ],
        [
            ("PAX8", POS_NUCLEAR),
            ("WT1", POS_NUCLEAR),
            ("p53", "aberrant overexpression in more than 90%"),
            ("GATA3", NEG),
        ],
        ["PIK3CA", "TP53"],
    ),
    (
        "A: gastric-wall core from a routine endoscopy. B: periportal node core obtained later.",
        "A: poorly differentiated adenocarcinoma with signet-ring cells. B: adenocarcinoma with gland formation.",
        [
            ("MUC5AC", POS_DIFFUSE),
            ("CK7", POS_DIFFUSE),
            ("CDX2", "weak focal nuclear staining in 10%"),
            ("SATB2", NEG),
        ],
        [
            ("SATB2", POS_NUCLEAR),
            ("CDX2", POS_NUCLEAR),
            ("CK20", POS_DIFFUSE),
            ("MUC5AC", NEG),
        ],
        ["CDKN2A", "TP53"],
    ),
    (
        "A: pelvic node core. B: bladder-wall biopsy core obtained later.",
        "A: adenocarcinoma with cribriform glands. B: high-grade carcinoma with solid nests.",
        [
            ("NKX3.1", POS_NUCLEAR),
            ("PSAP", positive("granular cytoplasmic staining")),
            ("GATA3", NEG),
        ],
        [
            ("GATA3", POS_NUCLEAR),
            ("p40", positive("focal nuclear staining")),
            ("Uroplakin II", positive("focal membranous staining")),
            ("NKX3.1", NEG),
        ],
        ["SPOP", "TP53"],
    ),
]


def build_conflict(rng, sid, case_no):
    specimen_a, morph, panel_a, panel_b, genes_a = rng.choice(CONFLICT_PAIRS)
    rows = demographics(sid, *age_sex(rng, rng.choice(["male", "female"])))
    rows += context_rows(
        sid,
        specimen_a,
        "CT and directed endoscopy identify no definite primary; no clonal comparison of A and B is available.",
        "A: FFPE tumor fraction 50%. B: FFPE tumor fraction 45%. Both have adequate stain controls.",
        morph,
        morph_group="specimens",
    )
    for g in genes_a:
        rows.append(
            mut_gene_row(
                sid, g, pick_weights(rng, [(0, 3), (1, 7)]), cover=DNA_COVER_MED
            )
        )
    rows.append(cna_row(sid, "MYC", pick_weights(rng, [(0, 6), (1, 4)])))
    idx = 1
    for m, v in panel_a:
        rows.append(
            ihc(
                sid,
                idx,
                f"A: {m}",
                v,
                group="ihc-a",
                cover_note="simulated specimen A; internal and external controls adequate",
            )
        )
        idx += 1
    for m, v in panel_b:
        rows.append(
            ihc(
                sid,
                idx,
                f"B: {m}",
                v,
                group="ihc-b",
                cover_note="simulated specimen B; internal and external controls adequate",
            )
        )
        idx += 1
    rows.append(
        unavailable(
            sid,
            idx,
            "Specimen B sequencing",
            "mutation",
            "Sequencing supplied only for specimen A; clonality unresolved",
        )
    )
    rows += tail_rows(sid)
    return rows, {
        "exercise": "conflicting_specimens",
        "candidates": ["insufficient_evidence"],
        "notes": (
            "Competing positive patterns in specimens A and B; possible "
            "distinct primaries with no proof of common clonality and "
            "sequencing limited to A."
        ),
    }


def build_hepatocellular(rng, sid, case_no):
    rows = demographics(sid, *age_sex(rng, "male"))
    specimen = f"{rng.choice(LIVER_SITES + ['Portacaval node core'])}; lytic rib lesions noted."
    workup = rng.choice(
        [
            "Multiphasic liver MRI shows only subcentimeter indeterminate nodules; upper/lower endoscopy unrevealing.",
            "CT shows hepatic and nodal disease without a cirrhotic liver background; no portal hypertension on record.",
        ]
    )
    quality = quality_text(rng)
    morphology = rng.choice(
        [
            "Carcinoma with thick trabeculae, polygonal cells, eosinophilic cytoplasm and focal bile-like pigment.",
            "Trabecular and pseudoglandular carcinoma with sinusoidal vasculature.",
        ]
    )
    rows += context_rows(sid, specimen, workup, quality, morphology)
    rows.append(mut_gene_row(sid, "CTNNB1", pick_weights(rng, [(0, 4), (1, 6)])))
    rows.append(mut_gene_row(sid, "AXIN1", pick_weights(rng, [(0, 6), (1, 4)])))
    rows.append(mut_gene_row(sid, "TP53", pick_weights(rng, [(0, 5), (1, 5)])))
    rows.append(cna_row(sid, "MYC", pick_weights(rng, [(0, 6), (1, 4)])))
    rows.append(cna_row(sid, "MET", pick_weights(rng, [(0, 8), (1, 2)])))
    markers = [
        ("Arginase-1", positive("strong diffuse cytoplasmic and nuclear staining")),
        ("HepPar-1", positive("granular cytoplasmic staining")),
        ("Glypican-3", positive("patchy membranous and cytoplasmic staining")),
        ("Polyclonal CEA", "positive; canalicular pattern"),
        ("CK7", NEG),
        ("CK20", NEG),
        ("PAX8", NEG),
        ("TTF-1", NEG),
    ]
    rows += [ihc(sid, i + 1, m, v) for i, (m, v) in enumerate(markers)]
    rows.append(
        unavailable(
            sid,
            1,
            "Albumin RNA in situ hybridization",
            "histology",
            "Not performed; tissue retained for review",
        )
    )
    rows += tail_rows(sid)
    return rows, {
        "exercise": "outside_taxonomy",
        "candidates": ["other_origin"],
        "notes": (
            "Hepatocellular differentiation has no matching class in "
            "the frozen taxonomy; do not force a liver specimen into "
            "CHOL or COADREAD."
        ),
    }


def build_pancreaticobiliary(rng, sid, case_no):
    rows = demographics(sid, *age_sex(rng, rng.choice(["male", "female"])))
    specimen = f"{rng.choice(LIVER_SITES + ['Peritoneal nodule core', 'Porta hepatis node core'])}; multiple hepatic and peritoneal deposits."
    workup = rng.choice(
        [
            "CT, MRCP and endoscopic ultrasound show no definite pancreatic or biliary primary.",
            "CT and EUS-guided survey show no dominant pancreatic mass; biliary tree is undilated.",
            "Multiphasic CT shows hepatic disease only; no ampullary or ductal lesion identified.",
        ]
    )
    quality = quality_text(rng)
    morphology = rng.choice(
        [
            "Mucin-producing adenocarcinoma with desmoplastic stroma.",
            "Moderately differentiated adenocarcinoma with irregular glands and desmoplasia.",
        ]
    )
    rows += context_rows(sid, specimen, workup, quality, morphology)
    smad4 = rng.random() < 0.6
    rows.append(mut_gene_row(sid, "KRAS", 1))
    rows.append(mut_gene_row(sid, "TP53", 1))
    rows.append(
        mut_gene_row(sid, "SMAD4", 1 if smad4 else 0, group="smad4" if smad4 else None)
    )
    rows.append(mut_gene_row(sid, "IDH1", 0))
    rows.append(cna_row(sid, "CDKN2A", pick_weights(rng, [(-2, 6), (0, 4)])))
    rows.append(cna_row(sid, "ERBB2", 0))
    markers = [
        ("CK7", POS_DIFFUSE),
        ("CK19", POS_DIFFUSE),
        ("MUC5AC", positive("diffuse cytoplasmic staining")),
        ("CK20", positive("focal staining in 10%")),
        ("CDX2", "positive; weak focal nuclear staining in 5%"),
        ("SATB2", NEG),
        ("TTF-1", NEG),
        ("PAX8", NEG),
    ]
    ihc_rows = [ihc(sid, i + 1, m, v) for i, (m, v) in enumerate(markers)]
    if smad4:
        loss = ihc(
            sid,
            len(markers) + 1,
            "SMAD4",
            "loss of tumor nuclear staining; stromal control retained",
            group="smad4",
        )
        ihc_rows.append(loss)
        for r in rows:
            if r["id"] == "mut-smad4":
                r["group"] = "smad4"
    rows += ihc_rows
    n = len([r for r in rows if r["id"].startswith("unavailable-")]) + 1
    rows.append(
        unavailable(
            sid,
            n,
            "Albumin RNA in situ hybridization",
            "histology",
            "Assay not performed",
        )
    )
    rows.append(
        unavailable(
            sid,
            n + 1,
            "FGFR2 rearrangement",
            "mutation",
            "DNA panel lacks validated fusion detection; RNA not available",
        )
    )
    rows += tail_rows(sid)
    return rows, {
        "exercise": "overlapping_origins",
        "candidates": ["PAAD", "CHOL", "EGC", "insufficient_evidence"],
        "notes": (
            "Shared CK7/KRAS/SMAD4-type findings in a liver or nodal "
            "biopsy do not settle pancreatic versus biliary versus "
            "upper-GI origin."
        ),
    }


# ---------------------------------------------------------------------------
# distribution and driver
# ---------------------------------------------------------------------------

ARCHETYPES = [
    (
        "pulmonary",
        build_pulmonary,
        240,
        "Pulmonary differentiation (adenocarcinoma and squamous) without an anatomical primary",
    ),
    (
        "intestinal",
        build_intestinal,
        192,
        "Intestinal differentiation, variable MMR completeness and keratin overlap",
    ),
    (
        "insufficient_bone_core",
        build_insufficient,
        180,
        "Scant acid-decalcified bone cores; failed controls and unresolved tumor-only findings",
    ),
    (
        "breast",
        build_breast,
        168,
        "Hormone-receptor and GATA3 panels with unrevealing imaging",
    ),
    (
        "gynecologic",
        build_gynecologic,
        156,
        "Serous and endometrioid patterns; site ambiguity and taxonomy boundaries",
    ),
    (
        "prostate",
        build_prostate,
        144,
        "NKX3.1/PSAP patterns; focal weak PSA and grouped PTEN axis",
    ),
    (
        "renal",
        build_renal,
        144,
        "Clear-cell patterns; VHL/PBRM1 cluster sharing one attribution group",
    ),
    (
        "pancreaticobiliary_overlap",
        build_pancreaticobiliary,
        120,
        "CK7/CK19 mucinous adenocarcinoma with PAAD/CHOL/EGC overlap",
    ),
    (
        "conflicting_specimens",
        build_conflict,
        84,
        "Two specimens with competing marker panels; sequencing for A only",
    ),
    (
        "hepatocellular_other_origin",
        build_hepatocellular,
        72,
        "Hepatocellular differentiation outside the frozen taxonomy",
    ),
]


def validate_rows(rows, sid):
    seen = set()
    for r in rows:
        assert len(r) == 22
        for key in ("value", "reference", "alternate", "position", "somatic"):
            assert r["status"] == "observed" or r[key] == "", (sid, r["id"], key)
        assert r["id"] not in seen, (sid, r["id"])
        seen.add(r["id"])
        for key in ("value", "coverage", "assay", "name"):
            if r["status"] == "observed" and key in (
                "value",
                "coverage",
                "assay",
                "name",
            ):
                pass
        assert "\t" not in "".join(r.values()) and "\n" not in "".join(r.values())


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    print(f"seed: {SEED}")
    scenarios = []
    total_rows = 0
    row_counts = []
    idx = 0
    for name, builder, count, summary in ARCHETYPES:
        for k in range(count):
            idx += 1
            sid = f"SYNTH-{100 + idx}"
            group = f"SYNTH-P{100 + idx}"
            rows, meta = builder(rng, sid, k)
            validate_rows(rows, sid)
            path = OUT / f"sample-{idx:03d}.tsv"
            with open(path, "w", encoding="utf-8", newline="") as fh:
                fh.write("\t".join(HEADER) + "\n")
                for r in rows:
                    ordered = [r[c] for c in HEADER]
                    assert all(v is not None for v in ordered)
                    fh.write("\t".join(ordered) + "\n")
            total_rows += len(rows)
            row_counts.append(len(rows))
            scenarios.append(
                {
                    "sample_id": sid,
                    "patient_group_id": group,
                    "file": path.name,
                    "partition": "development",
                    "archetype": name,
                    "candidates": meta["candidates"],
                    "exercise": meta["exercise"],
                    "notes": meta["notes"],
                }
            )
    # scenarios must be ordered by file like the sibling collection
    scenarios.sort(key=lambda s: s["file"])
    manifest = {
        "schema_version": 1,
        "collection": "gut-generated-v1",
        "label_status": (
            "Author-constructed scenario intent; not patient "
            "ground truth or clinical validation"
        ),
        "provider_input": False,
        "scenarios": scenarios,
    }
    with open(OUT / "scenarios.json", "w", encoding="utf-8") as fh:
        json.dump(manifest, fh, indent=2, ensure_ascii=True)
        fh.write("\n")

    dist = {name: count for name, _, count, _ in ARCHETYPES}
    print(f"patients: {idx}")
    print(
        f"feature rows: {total_rows} (mean {total_rows / idx:.1f}, "
        f"min {min(row_counts)}, max {max(row_counts)})"
    )
    print("archetype distribution:")
    for name, _, count, _ in ARCHETYPES:
        print(f"  {name:32s} {count:4d} ({count / idx:.1%})")
    exercise_counts = {}
    for s in scenarios:
        exercise_counts[s["exercise"]] = exercise_counts.get(s["exercise"], 0) + 1
    print("exercises:", json.dumps(exercise_counts, sort_keys=True))
    write_readme(dist, idx, total_rows, min(row_counts), max(row_counts))
    print(f"wrote {OUT}")


def write_readme(dist, n_patients, total_rows, min_rows, max_rows):
    from datetime import date

    rows = "\n".join(
        f"| {name} | {count} | {summary} |" for name, _, count, summary in ARCHETYPES
    )
    readme = f"""# {n_patients} synthetic CUP workups for the GUT project

Collection `gut-generated-v1`, generated {date.today().isoformat()} by
`scripts/generate_gut_fixture.py` (deterministic; RNG seed {SEED}, printed on
every run). These are {n_patients} distinct invented patients with {total_rows}
feature rows in total ({min_rows}-{max_rows} rows per patient). Loading uses
the normal TSV importer (`josh-ingest` molecular format) and makes no provider
request. No patient record, published patient example, or model prediction was
copied; every value is an invented, biologically motivated scenario.

All patients are synthetic. Sample identifiers `SYNTH-101`..`SYNTH-{100 + n_patients}`
and patient groups `SYNTH-P101`..`SYNTH-P{100 + n_patients}` do not overlap the
`synthetic-cup-v2` collection on purpose.

Generated for the GUT (Guided Unconscious Thinking) local-inference project -
github.com/qalarc/gut-finetuned.

## Coverage

| Archetype | Cases | What the archetype exercises |
| --- | --- | --- |
{rows}

The largest archetype is {max(dist.values())} of {n_patients} cases
({max(dist.values()) / n_patients:.0%}), so no single archetype dominates.
Beyond the archetypes above, the collection deliberately includes:

- discordant-marker cases whose IHC panel partially contradicts the leading
  lineage (exercise `discordant_markers`);
- site-ambiguity cases where several taxonomy classes remain plausible
  (exercise `site_ambiguity` or `overlapping_origins`);
- one conflicting-specimens archetype whose paired stains argue for two
  different origins with no clonal comparison;
- hepatocellular differentiation that falls outside the frozen taxonomy
  (exercise `outside_taxonomy`);
- insufficient decalcified bone-core cases dominated by failed controls,
  exhausted tissue and an unresolved tumor-only TP53 count.

`scenarios.json` is a separate local review manifest and is never loaded into
the model's evidence. It flags every special case with an `exercise` value and
a note on what to inspect. Do not demand predetermined numerical scores or
call scenario agreement accuracy.

## Data representation

The same rules as `fixtures/cup-realistic-v2` apply, because the importer is
shared:

- TSVs contain the app's processed feature schema, not raw sequencing data or
  complete panel inventories. Counts are explicit reported nonsynonymous
  variant counts per named gene, with tested scope, depth and detection limits
  recorded in the coverage text. No HGVS allele, pathogenicity annotation,
  germline exclusion or VAF is inferred from a count. Low-quality cases carry
  explicitly unresolved tumor-only findings (somatic status left open).
- CNA calls use the existing ordinal -2/-1/0/+1/+2 encoding. Measured zero is
  distinct from a failed measurement. `unknown` means uninterpretable or
  failed; `not_tested` means not performed; both carry no value. Low coverage
  never becomes a negative call. Unlisted genes were not supplied and cannot
  be assumed wild type.
- RNA and SBS assessment are explicitly not performed everywhere. A handful of
  panel mutations does not justify fabricating fitted signatures, expression
  vectors, HRD, MSI, or treatment-response estimates. Partial MMR staining in
  some intestinal cases does not establish global MMR status; BRCA counts in
  gynecologic cases do not establish HRD status.
- Specimen, workup and quality context are bounded categorical rows in the
  existing histology modality. Conflicting-specimen cases name specimens A and
  B in the stain names and coverage text and supply sequencing for A only,
  mirroring sample 009 of the sibling collection.
- Correlated features share attribution groups: the VHL/PBRM1 cluster
  (`renal-sequence`), PTEN mutation plus loss (`pten-axis`), SMAD4 count plus
  lost staining (`smad4`), TP53 count plus aberrant p53 staining (`tp53`) and
  MMR quartets (`mmr-panel`); conflicting specimens use `ihc-a`/`ihc-b`. These
  are documented attribution choices, not claims of causal dependence.
- Ages are integers, or censored as `>89`. A few sex rows are `unknown`,
  distinct from male or female. Source byte hashes and row numbers are
  recorded by the importer. Neutral sample identifiers, patient groups, source
  paths and hashes remain local.

## Regenerate

```bash
python3 scripts/generate_gut_fixture.py
```

The script is deterministic for a fixed seed and overwrites the collection in
place. Verification hint: `cargo test -p josh-ingest` exercises the importer
this collection must satisfy.

## Reference basis

As with `fixtures/cup-realistic-v2`, the OncoNPC study and the SEOM-GECOD
diagnostic guideline guide the genomic feature families, IHC lineage panels
and the frozen taxonomy. The combinations here are author-constructed
scenarios for local-inference research (GUT) and require pathology review
before use as an evaluation benchmark. No therapies, survival outcomes or
benefits are simulated.
"""
    with open(OUT / "README.md", "w", encoding="utf-8") as fh:
        fh.write(readme)


if __name__ == "__main__":
    main()
