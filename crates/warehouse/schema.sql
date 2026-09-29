-- The Tentzhen warehouse. Three layers, loaded in order by crates/warehouse:
--
--   bronze  every record file exactly as read; nothing parsed
--   silver  typed rows with keys, foreign keys and checks; each names the record it came from
--   gold    views that answer questions (marts); never stored, always derived
--
-- The records in parts/ and builds/ are the system of record. This database is rebuilt from them
-- on every load, so it can be deleted at any time.

CREATE SCHEMA bronze;
CREATE SCHEMA silver;
CREATE SCHEMA gold;

-- ---------------------------------------------------------------- bronze

CREATE TABLE bronze.record (
    path TEXT PRIMARY KEY,
    body TEXT NOT NULL
);

-- Lineage: the exact bytes each silver row came from.
CREATE VIEW bronze.record_hash AS
SELECT path, sha256(body) AS sha256 FROM bronze.record;

-- ---------------------------------------------------------------- silver

-- Each row's `record` names its bronze file. DuckDB refuses foreign keys across schemas, so that
-- link is held by the loader's test every_row_traces_to_the_bytes_it_came_from, not by a key.

CREATE TABLE silver.part (
    part            TEXT PRIMARY KEY,
    kind            TEXT NOT NULL CHECK (kind IN ('chip', 'module', 'product')),
    what            TEXT NOT NULL,
    authorized_only BOOLEAN NOT NULL,
    datasheet       TEXT,
    record          TEXT NOT NULL
);

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

CREATE TABLE silver.claim (
    build    TEXT NOT NULL,
    version  INTEGER NOT NULL,
    claim_no INTEGER NOT NULL,
    says     TEXT NOT NULL,
    PRIMARY KEY (build, version, claim_no),
    FOREIGN KEY (build, version) REFERENCES silver.build_version (build, version)
);

-- What shows a claim. A simulation grades tested, checked or proven; the bench grades measured.
CREATE TABLE silver.referent (
    build    TEXT NOT NULL,
    version  INTEGER NOT NULL,
    claim_no INTEGER NOT NULL,
    kind     TEXT NOT NULL CHECK (kind IN ('sim', 'irl')),
    grade    TEXT NOT NULL CHECK (grade IN ('tested', 'checked', 'proven', 'measured')),
    evidence TEXT NOT NULL,
    PRIMARY KEY (build, version, claim_no, kind),
    FOREIGN KEY (build, version, claim_no) REFERENCES silver.claim (build, version, claim_no),
    CHECK ((kind = 'irl') = (grade = 'measured'))
);

-- ---------------------------------------------------------------- gold

-- The newest version of each build.
CREATE VIEW gold.latest AS
SELECT build, max(version) AS version FROM silver.build_version GROUP BY build;

-- Every claim with both grades, derived from its referents. No referent reads 'unknown'.
CREATE VIEW gold.claim_grades AS
SELECT c.build, c.version, c.claim_no, c.says,
       coalesce(s.grade, 'unknown') AS sim_grade, s.evidence AS sim_evidence,
       coalesce(i.grade, 'unknown') AS irl_grade, i.evidence AS irl_evidence
FROM silver.claim c
LEFT JOIN silver.referent s
       ON s.build = c.build AND s.version = c.version AND s.claim_no = c.claim_no AND s.kind = 'sim'
LEFT JOIN silver.referent i
       ON i.build = c.build AND i.version = c.version AND i.claim_no = c.claim_no AND i.kind = 'irl';

-- How much of each build version is shown, and how.
CREATE VIEW gold.grade_coverage AS
SELECT build, version,
       count(*)                                                          AS claims,
       count(*) FILTER (WHERE sim_grade <> 'unknown')                    AS shown_in_sim,
       count(*) FILTER (WHERE irl_grade = 'measured')                    AS measured,
       count(*) FILTER (WHERE sim_grade = 'unknown' AND irl_grade = 'unknown') AS unknown
FROM gold.claim_grades
GROUP BY build, version;

-- Every part and commodity a build version needs, through every build it uses, with quantities
-- multiplied down the tree. The records forbid `uses` loops, so the recursion ends.
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

-- Where each part ends up, directly or inside another build.
CREATE VIEW gold.where_used AS
SELECT part, build, version, qty FROM gold.bom_exploded WHERE part IS NOT NULL;

-- ---------------------------------------------------------------- gold: the site's pages
-- Everything a build page shows comes from these views. The site reads nothing else.

-- One row per build version, with its newest sibling, its drawing and the hash of its record.
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

CREATE VIEW gold.page_line AS
SELECT l.build, l.version, l.line_no, l.qty, l.part, l.commodity, l.form,
       p.datasheet, coalesce(p.authorized_only, false) AS authorized_only
FROM silver.line l
LEFT JOIN silver.part p ON p.part = l.part;

CREATE VIEW gold.page_uses AS
SELECT build, version, uses_no, uses_build, uses_version, qty FROM silver.uses;

-- The same edges read the other way: where each build version is used.
CREATE VIEW gold.page_used_in AS
SELECT uses_build AS build, uses_version AS version,
       build AS in_build, version AS in_version, qty
FROM silver.uses;

CREATE VIEW gold.page_firmware AS SELECT * FROM silver.firmware;
CREATE VIEW gold.page_pin      AS SELECT * FROM silver.pin;
CREATE VIEW gold.page_step     AS SELECT * FROM silver.step;
