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
COMMENT ON TABLE bronze.record IS 'Every file the loader read, exactly as read: records, drawings, site documents, the lab''s limits, the lab-target list, the lab log and measurement records.';
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
    never_a_lab_target TEXT CHECK (trim(never_a_lab_target) <> ''),
    record  TEXT NOT NULL,
    PRIMARY KEY (build, version),
    CHECK (version = 1 OR changes IS NOT NULL)
);
COMMENT ON TABLE silver.build_version IS 'Each version of each build, from builds/<build>/v<n>.toml.';
COMMENT ON COLUMN silver.build_version.never_a_lab_target IS 'Why no board of this build, or of a build that uses it, may be a lab target; NULL if it may. crates/records refuses such a target; the database does not.';
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
    CHECK ((build IS NULL) = (version IS NULL)),
    -- Keys for a meter's accuracy, which must be a claim on the meter's own record.
    UNIQUE (claim, part),
    UNIQUE (claim, build, version)
);
COMMENT ON TABLE silver.claim IS 'What a part or a build version claims. Its grades come from its referents, never typed.';
COMMENT ON COLUMN silver.claim.claim IS 'How it is cited: <part>#<id>, or <build>/v<n>#<id>.';
COMMENT ON COLUMN silver.claim.claim_no IS 'Its place in its record, from 1.';

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
COMMENT ON TABLE silver.lab_limit_rests_on IS 'The claims a limit relies on to do its job.';

CREATE TABLE silver.lab_limit_at_most (
    path  TEXT PRIMARY KEY REFERENCES silver.lab_limit (path),
    claim TEXT NOT NULL,
    name  TEXT NOT NULL,
    FOREIGN KEY (path, claim) REFERENCES silver.lab_limit_rests_on (path, claim),
    FOREIGN KEY (claim, name) REFERENCES silver.claim_value (claim, name)
);
COMMENT ON TABLE silver.lab_limit_at_most IS 'A number a limit may not exceed: a value of a claim it rests on. crates/lab holds the limit to it.';
COMMENT ON COLUMN silver.lab_limit_at_most.name IS 'The claim''s value, in silver.claim_value.';

-- ---------------------------------------------------------------- silver: the lab-target list
-- From lab/targets.toml, which crates/records has checked, with each flash in the log against it.

CREATE TABLE silver.lab_target (
    target      TEXT PRIMARY KEY,
    part        TEXT REFERENCES silver.part (part),
    build       TEXT,
    version     INTEGER,
    serial      TEXT NOT NULL UNIQUE,
    listed      DATE NOT NULL,
    approved_by TEXT NOT NULL CHECK (regexp_full_match(approved_by, 'person:[a-z0-9-]+')),
    retired     DATE,
    record      TEXT NOT NULL,
    FOREIGN KEY (build, version) REFERENCES silver.build_version (build, version),
    CHECK ((part IS NULL) <> (build IS NULL)),
    CHECK ((build IS NULL) = (version IS NULL)),
    CHECK (retired > listed)
);
COMMENT ON TABLE silver.lab_target IS 'Each board an agent may flash without asking first: one physical board, a part or a build version. crates/records holds each flash in the lab log to it.';
COMMENT ON COLUMN silver.lab_target.target IS 'Its id, written on a label on the board.';
COMMENT ON COLUMN silver.lab_target.serial IS 'The serial the board reports, which no other target''s is.';
COMMENT ON COLUMN silver.lab_target.listed IS 'The first UTC day it may be flashed.';
COMMENT ON COLUMN silver.lab_target.approved_by IS 'On whose decision it was listed: a person''s, as person:<name>. The writer''s word.';
COMMENT ON COLUMN silver.lab_target.retired IS 'The UTC day it stops being a target, once it has.';
COMMENT ON COLUMN silver.lab_target.record IS 'The file in bronze.record this row came from.';

-- ---------------------------------------------------------------- silver: the lab log
-- From lab/log/<yyyy-mm-dd>.jsonl, which crates/records has checked, chain and all.

CREATE TABLE silver.lab_log_entry (
    seq           INTEGER PRIMARY KEY CHECK (seq >= 1),
    logged_at     TIMESTAMP NOT NULL,
    by_kind       TEXT NOT NULL CHECK (by_kind IN ('person', 'agent')),
    by_name       TEXT NOT NULL,
    what          TEXT NOT NULL,
    limits_sha256 TEXT NOT NULL CHECK (regexp_full_match(limits_sha256, '[0-9a-f]{64}')),
    sha256        TEXT NOT NULL UNIQUE CHECK (regexp_full_match(sha256, '[0-9a-f]{64}')),
    prev          TEXT UNIQUE REFERENCES silver.lab_log_entry (sha256),
    record        TEXT NOT NULL,
    CHECK ((seq = 1) = (prev IS NULL))
);
COMMENT ON TABLE silver.lab_log_entry IS 'Each hardware action in the lab log, numbered in the log''s order. Each entry names the sha256 of the line before it.';
COMMENT ON COLUMN silver.lab_log_entry.seq IS 'Its place in the log, from 1.';
COMMENT ON COLUMN silver.lab_log_entry.logged_at IS 'The entry''s `at`, in UTC.';
COMMENT ON COLUMN silver.lab_log_entry.by_kind IS 'Who acted, from the entry''s `by`: a person or an agent.';
COMMENT ON COLUMN silver.lab_log_entry.what IS 'The action, such as supply.set.';
COMMENT ON COLUMN silver.lab_log_entry.limits_sha256 IS 'The sha256 of lab/limits.toml as the host read it when the entry was written. Nothing checks that it names a version of that file.';
COMMENT ON COLUMN silver.lab_log_entry.sha256 IS 'The sha256 of the entry''s line, which the next entry names as its prev.';
COMMENT ON COLUMN silver.lab_log_entry.prev IS 'The sha256 of the line before it; NULL only for the first entry. Unique, so the log cannot fork.';
COMMENT ON COLUMN silver.lab_log_entry.record IS 'The file in bronze.record this row came from.';

CREATE TABLE silver.lab_log_value (
    seq    INTEGER NOT NULL REFERENCES silver.lab_log_entry (seq),
    side   TEXT NOT NULL CHECK (side IN ('params', 'result')),
    name   TEXT NOT NULL,
    amount DOUBLE,
    text   TEXT,
    flag   BOOLEAN,
    PRIMARY KEY (seq, side, name),
    CHECK ((amount IS NOT NULL)::INTEGER + (text IS NOT NULL)::INTEGER + (flag IS NOT NULL)::INTEGER = 1)
);
COMMENT ON TABLE silver.lab_log_value IS 'What each action was asked to do (params) and what happened (result): each a number, a word or a yes/no.';
COMMENT ON COLUMN silver.lab_log_value.name IS 'A number''s name ends in its unit: set_volts.';

-- ---------------------------------------------------------------- silver: measurement records
-- From lab/records/<id>.toml, which crates/records has checked against the catalogue and the log.

CREATE TABLE silver.measurement (
    measurement TEXT PRIMARY KEY,
    measures    TEXT NOT NULL,
    setup       TEXT NOT NULL,
    log_first   TEXT NOT NULL REFERENCES silver.lab_log_entry (sha256),
    log_last    TEXT NOT NULL REFERENCES silver.lab_log_entry (sha256),
    record      TEXT NOT NULL
);
COMMENT ON TABLE silver.measurement IS 'Each measurement record: one experiment on the bench, and the lab-log entries it came from. A claim''s record referent names one.';
COMMENT ON COLUMN silver.measurement.measurement IS 'Its id, the file''s name: <yyyy-mm-dd>-<words>, the UTC day of its first log entry.';
COMMENT ON COLUMN silver.measurement.measures IS 'What was measured.';
COMMENT ON COLUMN silver.measurement.setup IS 'How it was wired and set.';
COMMENT ON COLUMN silver.measurement.log_first IS 'Its first lab-log entry, by the sha256 of its line.';
COMMENT ON COLUMN silver.measurement.log_last IS 'Its last lab-log entry, by the sha256 of its line. crates/records holds it at or after the first.';
COMMENT ON COLUMN silver.measurement.record IS 'The file in bronze.record this row came from.';

-- DuckDB skips a foreign key when any of its columns is NULL, so each of the two accuracy keys
-- below binds only the kind of record the device is: a part, or a build version.
CREATE TABLE silver.measurement_device (
    measurement     TEXT NOT NULL REFERENCES silver.measurement (measurement),
    role            TEXT NOT NULL CHECK (role IN ('device', 'meter')),
    device_no       INTEGER NOT NULL,
    part            TEXT REFERENCES silver.part (part),
    build           TEXT,
    version         INTEGER,
    revision        TEXT,
    firmware_sha256 TEXT CHECK (regexp_full_match(firmware_sha256, '[0-9a-f]{64}')),
    gateware_sha256 TEXT CHECK (regexp_full_match(gateware_sha256, '[0-9a-f]{64}')),
    accuracy        TEXT,
    PRIMARY KEY (measurement, role, device_no),
    FOREIGN KEY (build, version) REFERENCES silver.build_version (build, version),
    FOREIGN KEY (accuracy, part) REFERENCES silver.claim (claim, part),
    FOREIGN KEY (accuracy, build, version) REFERENCES silver.claim (claim, build, version),
    CHECK ((part IS NULL) <> (build IS NULL)),
    CHECK ((build IS NULL) = (version IS NULL)),
    CHECK (revision IS NULL OR part IS NOT NULL),
    CHECK ((role = 'meter') = (accuracy IS NOT NULL))
);
COMMENT ON TABLE silver.measurement_device IS 'What each experiment measured (device) and what read it (meter): a part or a build version, with the hashes of the code it ran.';
COMMENT ON COLUMN silver.measurement_device.device_no IS 'Its place among the record''s devices, or its meters, from 1.';
COMMENT ON COLUMN silver.measurement_device.revision IS 'A part''s hardware revision, as marked on it. A build''s is its version.';
COMMENT ON COLUMN silver.measurement_device.accuracy IS 'A meter''s: the claim on its own record that gives its accuracy.';

CREATE TABLE silver.measurement_tool (
    measurement TEXT NOT NULL REFERENCES silver.measurement (measurement),
    tool        TEXT NOT NULL,
    version     TEXT NOT NULL,
    PRIMARY KEY (measurement, tool)
);
COMMENT ON TABLE silver.measurement_tool IS 'The tools each experiment used, and their versions.';

CREATE TABLE silver.measurement_value (
    measurement TEXT NOT NULL REFERENCES silver.measurement (measurement),
    name        TEXT NOT NULL,
    amount      DOUBLE,
    text        TEXT,
    flag        BOOLEAN,
    PRIMARY KEY (measurement, name),
    CHECK ((amount IS NOT NULL)::INTEGER + (text IS NOT NULL)::INTEGER + (flag IS NOT NULL)::INTEGER = 1)
);
COMMENT ON TABLE silver.measurement_value IS 'What each experiment found: each a number, a word or a yes/no.';
COMMENT ON COLUMN silver.measurement_value.name IS 'A number''s name ends in its unit: input_volts. A raw ADC code, input_codes, sits beside the value converted from it; crates/records holds that.';

-- ---------------------------------------------------------------- silver: the lab's sourcing
-- From lab/sourcing.toml, which crates/records has checked against the builds and the measurement
-- records. A line names a build's line by its part and form, or its commodity and form.
-- crates/records refuses two lines for one thing; the database does not.

CREATE TABLE silver.sourcing (
    line_no         INTEGER PRIMARY KEY,
    part            TEXT REFERENCES silver.part (part),
    form            TEXT,
    commodity       TEXT,
    shop            TEXT CHECK (shop IN ('aliexpress', 'amazon', 'lcsc', 'taobao')),
    store           TEXT,
    item            TEXT,
    listing_checked DATE,
    in_cart         DATE,
    ordered         DATE,
    arrived         DATE,
    passed_qa       TEXT REFERENCES silver.measurement (measurement),
    record          TEXT NOT NULL,
    CHECK ((part IS NULL) <> (commodity IS NULL)),
    CHECK ((shop IS NULL) = (item IS NULL)),
    CHECK (store IS NULL OR shop IS NOT NULL),
    CHECK (shop IS NOT NULL OR coalesce(listing_checked, in_cart, ordered) IS NULL),
    CHECK (listing_checked <= in_cart AND listing_checked <= ordered AND listing_checked <= arrived
           AND in_cart <= ordered AND in_cart <= arrived AND ordered <= arrived)
);
COMMENT ON TABLE silver.sourcing IS 'Where the lab buys each thing its builds need, and the UTC day it reached each stage. A thing with no row, or a row that has reached no stage, is specced.';
COMMENT ON COLUMN silver.sourcing.line_no IS 'Its place in the file, from 1.';
COMMENT ON COLUMN silver.sourcing.store IS 'The seller, as the listing names it.';
COMMENT ON COLUMN silver.sourcing.item IS 'The shop''s item number: on Amazon the ASIN, on LCSC the C number. crates/records holds its shape.';
COMMENT ON COLUMN silver.sourcing.listing_checked IS 'The UTC day a person checked the listing against the build.';
COMMENT ON COLUMN silver.sourcing.passed_qa IS 'The measurement record of the incoming QA it passed. crates/records holds that it lists the part and is from on or after the day it arrived; the database holds that it exists.';
COMMENT ON COLUMN silver.sourcing.record IS 'The file in bronze.record this row came from.';

-- ---------------------------------------------------------------- silver: what shows a claim
-- After the measurement records, which a record referent names.

CREATE TABLE silver.referent (
    claim       TEXT NOT NULL REFERENCES silver.claim (claim),
    kind        TEXT NOT NULL CHECK (kind IN ('proof', 'check', 'test', 'record', 'trusted')),
    evidence    TEXT,
    trusted     TEXT REFERENCES silver.trusted_entry (entry),
    measurement TEXT REFERENCES silver.measurement (measurement),
    PRIMARY KEY (claim, kind),
    CHECK ((kind = 'trusted') = (trusted IS NOT NULL)),
    CHECK ((kind = 'record') = (measurement IS NOT NULL)),
    CHECK ((evidence IS NULL) = (kind IN ('trusted', 'record')))
);
COMMENT ON TABLE silver.referent IS 'What shows a claim: a proof, a bounded check or a test in simulation; a bench measurement record; or an entry in the trusted base.';
COMMENT ON COLUMN silver.referent.evidence IS 'The proof, check or test, as the claim names it. Nothing checks that it exists.';
COMMENT ON COLUMN silver.referent.trusted IS 'For a trusted referent, the entry in the trusted base that assumes the claim.';
COMMENT ON COLUMN silver.referent.measurement IS 'For a record referent, the measurement record that shows the claim. crates/records holds that the record measured the claim''s subject.';

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
           max(measurement) FILTER (WHERE kind = 'record') AS record,
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

CREATE VIEW gold.claim_values AS
SELECT claim, name, amount FROM silver.claim_value;
COMMENT ON VIEW gold.claim_values IS 'The numbers each claim states, for readers that write them into the claim''s words.';

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

CREATE VIEW gold.build_tree AS
WITH RECURSIVE tree (root_build, root_version, build, version, qty) AS (
    SELECT build, version, build, version, 1 FROM silver.build_version
    UNION ALL
    SELECT t.root_build, t.root_version, u.uses_build, u.uses_version, t.qty * u.qty
    FROM tree t
    JOIN silver.uses u ON u.build = t.build AND u.version = t.version
)
SELECT root_build, root_version, build, version, sum(qty) AS qty FROM tree GROUP BY ALL;
COMMENT ON VIEW gold.build_tree IS 'Every build version each build version is made of, itself included, and how many of it, through every build it uses. The records forbid `uses` loops, so the recursion ends.';

CREATE VIEW gold.bom_exploded AS
SELECT t.root_build AS build, t.root_version AS version,
       l.part, l.commodity, l.form, sum(l.qty * t.qty) AS qty
FROM gold.build_tree t
JOIN silver.line l ON l.build = t.build AND l.version = t.version
GROUP BY ALL;
COMMENT ON VIEW gold.bom_exploded IS 'Every part and commodity a build version needs, through every build it uses, with quantities multiplied down the tree.';

CREATE VIEW gold.where_used AS
SELECT part, build, version, qty FROM gold.bom_exploded WHERE part IS NOT NULL;
COMMENT ON VIEW gold.where_used IS 'Where each part ends up, directly or inside another build.';

CREATE VIEW gold.bench AS
SELECT l.build, l.version
FROM gold.latest l
WHERE NOT EXISTS (
    SELECT 1 FROM silver.uses u
    JOIN gold.latest p ON p.build = u.build AND p.version = u.version
    WHERE u.uses_build = l.build AND u.uses_version = l.version
);
COMMENT ON VIEW gold.bench IS 'The build versions the lab builds, each once: the newest version of every build, less one that the newest version of another build uses, which is built inside that one.';

CREATE VIEW gold.bench_line AS
WITH needed AS (
    SELECT t.build, t.version, l.line_no, l.part, l.form, l.commodity, sum(l.qty * t.qty) AS qty
    FROM gold.bench b
    JOIN gold.build_tree t ON t.root_build = b.build AND t.root_version = b.version
    JOIN silver.line l ON l.build = t.build AND l.version = t.version
    GROUP BY ALL
),
ranked AS (
    SELECT *, row_number() OVER (ORDER BY build, version, line_no) AS n FROM needed
)
SELECT build, version, line_no, part, form, commodity, qty,
       min(n) OVER (PARTITION BY part, form, commodity) AS parts_no
FROM ranked;
COMMENT ON VIEW gold.bench_line IS 'Every line the bench needs, in the build version that writes it, with how many: its qty times how many of that version the builds in gold.bench take. parts_no is its row on the parts list, shared by every line that buys the same thing (part and form, or commodity and form): the place of the first such line, by build, version and line.';

CREATE VIEW gold.sourcing_status AS
SELECT s.line_no, s.part, s.form, s.commodity, s.shop, s.store, s.item,
       CASE WHEN s.passed_qa IS NOT NULL       THEN 'passed incoming QA'
            WHEN s.arrived IS NOT NULL         THEN 'arrived'
            WHEN s.ordered IS NOT NULL         THEN 'ordered'
            WHEN s.in_cart IS NOT NULL         THEN 'in cart'
            WHEN s.listing_checked IS NOT NULL THEN 'listing checked'
            ELSE 'specced' END                 AS status,
       CASE WHEN s.passed_qa IS NOT NULL THEN CAST(e.logged_at AS DATE)
            ELSE coalesce(s.arrived, s.ordered, s.in_cart, s.listing_checked) END AS since,
       s.passed_qa
FROM silver.sourcing s
LEFT JOIN silver.measurement m    ON m.measurement = s.passed_qa
LEFT JOIN silver.lab_log_entry e  ON e.sha256 = m.log_first;
COMMENT ON VIEW gold.sourcing_status IS 'Each line of the lab''s sourcing with its status, the last stage it has reached, never typed, and since, the UTC day it reached it: for incoming QA, the day of its record''s first lab-log entry; NULL while specced.';

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

CREATE VIEW gold.page_parts AS
SELECT b.parts_no, sum(b.qty) AS qty, b.part, b.form, b.commodity,
       p.datasheet, coalesce(p.authorized_only, false) AS authorized_only,
       s.shop, s.store, s.item, coalesce(s.status, 'specced') AS status, s.since, s.passed_qa
FROM gold.bench_line b
LEFT JOIN silver.part p ON p.part = b.part
LEFT JOIN gold.sourcing_status s
       ON s.part IS NOT DISTINCT FROM b.part
      AND s.form IS NOT DISTINCT FROM b.form
      AND s.commodity IS NOT DISTINCT FROM b.commodity
GROUP BY ALL;
COMMENT ON VIEW gold.page_parts IS 'The parts page: each thing the bench needs once, with how many, where the lab buys it and how far it has got. No prices: CLAUDE.md keeps them out.';

CREATE VIEW gold.page_parts_used_in AS
SELECT parts_no, build, version, sum(qty) AS qty FROM gold.bench_line GROUP BY ALL;
COMMENT ON VIEW gold.page_parts_used_in IS 'The build versions that write each row of the parts page, and how many each needs.';

CREATE VIEW gold.legal AS
SELECT body FROM bronze.record WHERE path = 'DISCLAIMER.md';
COMMENT ON VIEW gold.legal IS 'The disclaimer, as the site''s legal page shows it.';
