-- The Tentzhen warehouse. Three layers, loaded in order by crates/warehouse:
--
--   bronze  every record file exactly as read; nothing parsed
--   silver  typed rows with keys, foreign keys and checks; each names the record it came from
--   gold    views that answer questions (marts); never stored, always derived
--
-- The records in parts/, builds/ and lab/ are the system of record. This database is rebuilt
-- from them on every load, so it can be deleted at any time.
--
-- Every table and view says what it is in a COMMENT ON, which docs/data-catalogue.md is generated
-- from (`cargo run -p tentzhen-warehouse -- catalogue`).

CREATE SCHEMA bronze;
CREATE SCHEMA silver;
CREATE SCHEMA gold;

-- ---------------------------------------------------------------- bronze

CREATE TABLE bronze.record (
    path TEXT PRIMARY KEY,
    body TEXT NOT NULL
);
COMMENT ON TABLE bronze.record IS 'Every file the loader read, exactly as read: records, drawings, site documents and the lab''s limits.';
COMMENT ON COLUMN bronze.record.path IS 'The file''s path, relative to the repo root.';

CREATE VIEW bronze.record_hash AS
SELECT path, sha256(body) AS sha256 FROM bronze.record;
COMMENT ON VIEW bronze.record_hash IS 'Lineage: the sha256 of each file''s text.';

-- ---------------------------------------------------------------- silver

-- A table with a `record` column names its bronze file there; every other silver table references
-- one that does. DuckDB refuses foreign keys across schemas, so the link to bronze is held by the
-- loader's test every_row_traces_to_the_bytes_it_came_from, not by a key.

CREATE TABLE silver.part (
    part            TEXT PRIMARY KEY,
    kind            TEXT NOT NULL CHECK (kind IN ('chip', 'module', 'product')),
    what            TEXT NOT NULL,
    authorized_only BOOLEAN NOT NULL,
    datasheet       TEXT,
    record          TEXT NOT NULL
);
COMMENT ON TABLE silver.part IS 'Each part record in parts/.';
COMMENT ON COLUMN silver.part.what IS 'The record''s `is`: what the part is.';
COMMENT ON COLUMN silver.part.record IS 'The file in bronze.record this row came from.';

CREATE TABLE silver.build_version (
    build   TEXT NOT NULL,
    version INTEGER NOT NULL CHECK (version >= 1),
    status  TEXT NOT NULL CHECK (status IN ('draft', 'published')),
    does    TEXT NOT NULL,
    changes TEXT,
    drawing TEXT,
    record  TEXT NOT NULL,
    PRIMARY KEY (build, version),
    CHECK (version = 1 OR changes IS NOT NULL)
);
COMMENT ON TABLE silver.build_version IS 'Each version of each build, from builds/<build>/v<n>.toml.';
COMMENT ON COLUMN silver.build_version.record IS 'The file in bronze.record this row came from.';

CREATE TABLE silver.line (
    build     TEXT NOT NULL,
    version   INTEGER NOT NULL,
    line_no   INTEGER NOT NULL,
    part      TEXT REFERENCES silver.part (part),
    commodity TEXT,
    form      TEXT,
    qty       INTEGER NOT NULL CHECK (qty > 0),
    PRIMARY KEY (build, version, line_no),
    FOREIGN KEY (build, version) REFERENCES silver.build_version (build, version),
    CHECK ((part IS NULL) <> (commodity IS NULL))
);
COMMENT ON TABLE silver.line IS 'A build version''s bill of materials: a part or a commodity on each line, and how many.';

CREATE TABLE silver.uses (
    build        TEXT NOT NULL,
    version      INTEGER NOT NULL,
    uses_no      INTEGER NOT NULL,
    uses_build   TEXT NOT NULL,
    uses_version INTEGER NOT NULL,
    qty          INTEGER NOT NULL CHECK (qty > 0),
    PRIMARY KEY (build, version, uses_build, uses_version),
    FOREIGN KEY (build, version) REFERENCES silver.build_version (build, version),
    FOREIGN KEY (uses_build, uses_version) REFERENCES silver.build_version (build, version)
);
COMMENT ON TABLE silver.uses IS 'Another build version a build version is made from, and how many.';

CREATE TABLE silver.firmware (
    build   TEXT NOT NULL,
    version INTEGER NOT NULL,
    name    TEXT NOT NULL,
    license TEXT NOT NULL,
    source  TEXT NOT NULL,
    release TEXT NOT NULL,
    file    TEXT NOT NULL,
    sha256  TEXT NOT NULL CHECK (regexp_full_match(sha256, '[0-9a-f]{64}')),
    pin_map TEXT,
    PRIMARY KEY (build, version),
    FOREIGN KEY (build, version) REFERENCES silver.build_version (build, version)
);
COMMENT ON TABLE silver.firmware IS 'The firmware a build version runs, pinned by its sha256.';

CREATE TABLE silver.pin (
    build     TEXT NOT NULL,
    version   INTEGER NOT NULL,
    pin_no    INTEGER NOT NULL,
    name      TEXT NOT NULL,
    board_pin INTEGER NOT NULL,
    net       TEXT NOT NULL,
    required  BOOLEAN NOT NULL,
    PRIMARY KEY (build, version, pin_no),
    FOREIGN KEY (build, version) REFERENCES silver.build_version (build, version)
);
COMMENT ON TABLE silver.pin IS 'A build version''s pin assignments.';

CREATE TABLE silver.step (
    build       TEXT NOT NULL,
    version     INTEGER NOT NULL,
    step_no     INTEGER NOT NULL,
    instruction TEXT NOT NULL,
    run         TEXT,
    expect      TEXT,
    agent       TEXT,
    PRIMARY KEY (build, version, step_no),
    FOREIGN KEY (build, version) REFERENCES silver.build_version (build, version)
);
COMMENT ON TABLE silver.step IS 'A build version''s steps, in order.';
COMMENT ON COLUMN silver.step.instruction IS 'The record''s `do`.';

CREATE TABLE silver.trusted_entry (
    entry    TEXT PRIMARY KEY,
    entry_no INTEGER NOT NULL,
    assumes  TEXT NOT NULL,
    record   TEXT NOT NULL
);
COMMENT ON TABLE silver.trusted_entry IS 'The trusted base, from trusted-base.toml: what is assumed, not shown. A trusted referent names an entry by its id.';
COMMENT ON COLUMN silver.trusted_entry.entry IS 'Its id.';
COMMENT ON COLUMN silver.trusted_entry.entry_no IS 'Its place in the file, from 1.';
COMMENT ON COLUMN silver.trusted_entry.record IS 'The file in bronze.record this row came from.';

-- DuckDB skips a foreign key when any of its columns is NULL, so the checks below also require
-- build and version together.
CREATE TABLE silver.claim (
    claim    TEXT PRIMARY KEY,
    part     TEXT REFERENCES silver.part (part),
    build    TEXT,
    version  INTEGER,
    claim_no INTEGER NOT NULL,
    id       TEXT NOT NULL,
    says     TEXT NOT NULL,
    FOREIGN KEY (build, version) REFERENCES silver.build_version (build, version),
    CHECK ((part IS NULL) <> (build IS NULL)),
    CHECK ((build IS NULL) = (version IS NULL))
);
COMMENT ON TABLE silver.claim IS 'What a part or a build version claims. Its grades come from its referents, never typed.';
COMMENT ON COLUMN silver.claim.claim IS 'How it is cited: <part>#<id>, or <build>/v<n>#<id>.';
COMMENT ON COLUMN silver.claim.claim_no IS 'Its place in its record, from 1.';

CREATE TABLE silver.referent (
    claim    TEXT NOT NULL REFERENCES silver.claim (claim),
    kind     TEXT NOT NULL CHECK (kind IN ('proof', 'check', 'test', 'record', 'trusted')),
    evidence TEXT,
    trusted  TEXT REFERENCES silver.trusted_entry (entry),
    PRIMARY KEY (claim, kind),
    CHECK ((kind = 'trusted') = (trusted IS NOT NULL)),
    CHECK ((evidence IS NULL) = (trusted IS NOT NULL))
);
COMMENT ON TABLE silver.referent IS 'What shows a claim: a proof, a bounded check or a test in simulation; a bench measurement record; or an entry in the trusted base.';
COMMENT ON COLUMN silver.referent.evidence IS 'The proof, check, test or measurement record, as the claim names it. Nothing checks that it exists.';
COMMENT ON COLUMN silver.referent.trusted IS 'For a trusted referent, the entry in the trusted base that assumes the claim.';

CREATE TABLE silver.claim_value (
    claim  TEXT NOT NULL REFERENCES silver.claim (claim),
    name   TEXT NOT NULL,
    amount DOUBLE NOT NULL,
    PRIMARY KEY (claim, name)
);
COMMENT ON TABLE silver.claim_value IS 'The numbers a claim states.';
COMMENT ON COLUMN silver.claim_value.name IS 'Ends in its unit: min_volts, input_ratio.';

-- ---------------------------------------------------------------- silver: the lab's limits
-- From lab/limits.toml, which crates/lab has checked.

CREATE TABLE silver.lab_limit (
    path   TEXT PRIMARY KEY,
    amount DOUBLE,
    unit   TEXT CHECK (unit IN ('V', 'A')),
    choice TEXT,
    basis  TEXT NOT NULL CHECK (basis IN ('policy', 'same_as', 'rule')),
    policy TEXT,
    rule   TEXT,
    record TEXT NOT NULL,
    CHECK ((amount IS NULL) = (unit IS NULL)),
    CHECK ((amount IS NULL) <> (choice IS NULL)),
    CHECK ((basis = 'policy') = (policy IS NOT NULL)),
    CHECK ((basis = 'rule') = (rule IS NOT NULL))
);
COMMENT ON TABLE silver.lab_limit IS 'Each limit and its basis: a person''s decision (policy), a copy of another value (same_as), or a rule in crates/lab (rule).';
COMMENT ON COLUMN silver.lab_limit.path IS 'Where the value sits in the file, as table.key.';
COMMENT ON COLUMN silver.lab_limit.record IS 'The file in bronze.record this row came from.';

CREATE TABLE silver.lab_limit_input (
    path  TEXT NOT NULL REFERENCES silver.lab_limit (path),
    input TEXT NOT NULL REFERENCES silver.lab_limit (path),
    PRIMARY KEY (path, input)
);
COMMENT ON TABLE silver.lab_limit_input IS 'The values each limit comes from: the original it copies, or its rule''s inputs.';

CREATE TABLE silver.lab_limit_rests_on (
    path  TEXT NOT NULL REFERENCES silver.lab_limit (path),
    claim TEXT NOT NULL REFERENCES silver.claim (claim),
    PRIMARY KEY (path, claim)
);
COMMENT ON TABLE silver.lab_limit_rests_on IS 'The claims a limit relies on to do its job, on the parts they are about.';

-- ---------------------------------------------------------------- gold

CREATE VIEW gold.latest AS
SELECT build, max(version) AS version FROM silver.build_version GROUP BY build;
COMMENT ON VIEW gold.latest IS 'The newest version of each build.';

CREATE VIEW gold.claim_grades AS
WITH r AS (
    SELECT claim,
           max(evidence) FILTER (WHERE kind = 'proof')   AS proof,
           max(evidence) FILTER (WHERE kind = 'check')   AS "check",
           max(evidence) FILTER (WHERE kind = 'test')    AS test,
           max(evidence) FILTER (WHERE kind = 'record')  AS record,
           max(trusted)  FILTER (WHERE kind = 'trusted') AS trusted
    FROM silver.referent GROUP BY claim
)
SELECT c.claim, c.part, c.build, c.version, c.claim_no, c.id, c.says,
       CASE WHEN r.proof IS NOT NULL THEN 'proven'
            WHEN r."check" IS NOT NULL THEN 'checked'
            WHEN r.test IS NOT NULL THEN 'tested'
            ELSE 'unknown' END                   AS sim_grade,
       coalesce(r.proof, r."check", r.test)     AS sim_evidence,
       CASE WHEN r.record IS NOT NULL THEN 'measured'
            WHEN r.trusted IS NOT NULL THEN 'trusted'
            ELSE 'unknown' END                   AS bench_grade,
       coalesce(r.record, r.trusted)            AS bench_evidence
FROM silver.claim c
LEFT JOIN r USING (claim);
COMMENT ON VIEW gold.claim_grades IS 'Every claim on a part or a build version, with its grade on each axis: in simulation (proven, checked, tested) and on the bench (measured, trusted), each the strongest its referents give. No referent reads ''unknown''.';

CREATE VIEW gold.grade_coverage AS
SELECT coalesce(part, build || '/v' || version)                            AS subject,
       count(*)                                                            AS claims,
       count(*) FILTER (WHERE sim_grade <> 'unknown')                      AS shown_in_sim,
       count(*) FILTER (WHERE bench_grade = 'measured')                    AS measured,
       count(*) FILTER (WHERE bench_grade = 'trusted')                     AS trusted,
       count(*) FILTER (WHERE sim_grade = 'unknown' AND bench_grade = 'unknown') AS unknown
FROM gold.claim_grades
GROUP BY ALL;
COMMENT ON VIEW gold.grade_coverage IS 'How much of each part and build version is shown, and how.';

CREATE VIEW gold.bom_exploded AS
WITH RECURSIVE tree (root_build, root_version, build, version, mult) AS (
    SELECT build, version, build, version, 1 FROM silver.build_version
    UNION ALL
    SELECT t.root_build, t.root_version, u.uses_build, u.uses_version, t.mult * u.qty
    FROM tree t
    JOIN silver.uses u ON u.build = t.build AND u.version = t.version
)
SELECT t.root_build AS build, t.root_version AS version,
       l.part, l.commodity, l.form, sum(l.qty * t.mult) AS qty
FROM tree t
JOIN silver.line l ON l.build = t.build AND l.version = t.version
GROUP BY ALL;
COMMENT ON VIEW gold.bom_exploded IS 'Every part and commodity a build version needs, through every build it uses, with quantities multiplied down the tree. The records forbid `uses` loops, so the recursion ends.';

CREATE VIEW gold.where_used AS
SELECT part, build, version, qty FROM gold.bom_exploded WHERE part IS NOT NULL;
COMMENT ON VIEW gold.where_used IS 'Where each part ends up, directly or inside another build.';

CREATE VIEW gold.limit_grades AS
WITH RECURSIVE reach (path, via) AS (
    SELECT path, path FROM silver.lab_limit
    UNION
    SELECT r.path, i.input FROM reach r JOIN silver.lab_limit_input i ON i.path = r.via
),
rests AS (
    SELECT r.path, g.claim, g.bench_grade AS grade,
           CASE g.bench_grade WHEN 'unknown' THEN 0 WHEN 'trusted' THEN 1 ELSE 2 END AS strength
    FROM reach r
    JOIN silver.lab_limit_rests_on o ON o.path = r.via
    JOIN gold.claim_grades g ON g.claim = o.claim
)
SELECT l.path, l.amount, l.unit, l.choice, l.basis,
       arg_min(s.grade, s.strength)                     AS weakest_grade,
       string_agg(DISTINCT s.claim, ', ' ORDER BY s.claim) AS rests_on
FROM silver.lab_limit l
LEFT JOIN rests s USING (path)
GROUP BY l.path, l.amount, l.unit, l.choice, l.basis;
COMMENT ON VIEW gold.limit_grades IS 'Each limit with the weakest bench grade (measured, trusted) among the claims it rests on, directly or through any value it comes from. NULL when it rests on no claim: nothing here claims it does its job.';

-- ---------------------------------------------------------------- gold: the site's pages
-- Views that exist for the site's pages. The site also reads gold.claim_grades above, and reads
-- only gold: the test the_site_reads_only_gold holds that.

CREATE VIEW gold.page AS
SELECT bv.build, bv.version, bv.status, bv.does, bv.changes,
       l.version   AS latest,
       lv.changes  AS latest_changes,
       d.body      AS drawing_svg,
       bv.record,
       h.sha256    AS record_sha256
FROM silver.build_version bv
JOIN gold.latest l          ON l.build = bv.build
JOIN silver.build_version lv ON lv.build = l.build AND lv.version = l.version
JOIN bronze.record_hash h   ON h.path = bv.record
LEFT JOIN bronze.record d   ON d.path = 'builds/' || bv.build || '/' || bv.drawing;
COMMENT ON VIEW gold.page IS 'One row per build version, with its newest sibling, its drawing and the hash of its record.';

CREATE VIEW gold.page_line AS
SELECT l.build, l.version, l.line_no, l.qty, l.part, l.commodity, l.form,
       p.datasheet, coalesce(p.authorized_only, false) AS authorized_only
FROM silver.line l
LEFT JOIN silver.part p ON p.part = l.part;
COMMENT ON VIEW gold.page_line IS 'A build page''s bill of materials, with each part''s datasheet and whether it must come from an authorized distributor.';

CREATE VIEW gold.page_uses AS
SELECT build, version, uses_no, uses_build, uses_version, qty FROM silver.uses;
COMMENT ON VIEW gold.page_uses IS 'The build versions a build version is made from, for its page.';

CREATE VIEW gold.page_used_in AS
SELECT uses_build AS build, uses_version AS version,
       build AS in_build, version AS in_version, qty
FROM silver.uses;
COMMENT ON VIEW gold.page_used_in IS 'silver.uses read the other way: where each build version is used.';

CREATE VIEW gold.page_firmware AS SELECT * FROM silver.firmware;
CREATE VIEW gold.page_pin      AS SELECT * FROM silver.pin;
CREATE VIEW gold.page_step     AS SELECT * FROM silver.step;
COMMENT ON VIEW gold.page_firmware IS 'silver.firmware, for build pages.';
COMMENT ON VIEW gold.page_pin IS 'silver.pin, for build pages.';
COMMENT ON VIEW gold.page_step IS 'silver.step, for build pages.';

CREATE VIEW gold.legal AS
SELECT body FROM bronze.record WHERE path = 'DISCLAIMER.md';
COMMENT ON VIEW gold.legal IS 'The disclaimer, as the site''s legal page shows it.';
