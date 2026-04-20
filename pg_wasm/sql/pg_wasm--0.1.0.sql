SET allow_system_table_mods = on;
SELECT pg_catalog.set_config('allow_system_table_mods', 'on', false);

CREATE SCHEMA IF NOT EXISTS pg_wasm;
RESET allow_system_table_mods;

CREATE TABLE IF NOT EXISTS pg_wasm.modules (
    module_id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    abi TEXT NOT NULL,
    digest BYTEA NOT NULL,
    wasm_sha256 BYTEA NOT NULL,
    origin TEXT NOT NULL,
    artifact_path TEXT NOT NULL,
    wit_world TEXT NOT NULL,
    policy JSONB NOT NULL DEFAULT '{}'::jsonb,
    limits JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    generation BIGINT NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS pg_wasm.exports (
    export_id BIGSERIAL PRIMARY KEY,
    module_id BIGINT NOT NULL REFERENCES pg_wasm.modules (module_id) ON DELETE CASCADE,
    wasm_name TEXT NOT NULL,
    sql_name TEXT NOT NULL,
    signature JSONB NOT NULL DEFAULT '{}'::jsonb,
    arg_types OID[] NOT NULL DEFAULT ARRAY[]::oid[],
    ret_type OID,
    fn_oid OID,
    kind TEXT NOT NULL,
    UNIQUE (module_id, wasm_name),
    UNIQUE (module_id, sql_name)
);

CREATE TABLE IF NOT EXISTS pg_wasm.wit_types (
    wit_type_id BIGSERIAL PRIMARY KEY,
    module_id BIGINT NOT NULL REFERENCES pg_wasm.modules (module_id) ON DELETE CASCADE,
    wit_name TEXT NOT NULL,
    pg_type_oid OID NOT NULL,
    kind TEXT NOT NULL,
    definition JSONB NOT NULL DEFAULT '{}'::jsonb,
    UNIQUE (module_id, wit_name),
    UNIQUE (module_id, pg_type_oid)
);

CREATE TABLE IF NOT EXISTS pg_wasm.dependencies (
    module_id BIGINT NOT NULL REFERENCES pg_wasm.modules (module_id) ON DELETE CASCADE,
    depends_on_module_id BIGINT NOT NULL REFERENCES pg_wasm.modules (module_id) ON DELETE CASCADE,
    PRIMARY KEY (module_id, depends_on_module_id),
    CHECK (module_id <> depends_on_module_id)
);

DO $$
DECLARE
    loader_role TEXT;
    reader_role TEXT;
BEGIN
    -- PostgreSQL reserves pg_* role names; fall back to pgwasm_* where required.
    BEGIN
        IF NOT EXISTS (SELECT 1 FROM pg_catalog.pg_roles WHERE rolname = 'pg_wasm_loader') THEN
            CREATE ROLE pg_wasm_loader NOLOGIN;
        END IF;
        loader_role := 'pg_wasm_loader';
    EXCEPTION
        WHEN SQLSTATE '42939' THEN
            IF NOT EXISTS (SELECT 1 FROM pg_catalog.pg_roles WHERE rolname = 'pgwasm_loader') THEN
                CREATE ROLE pgwasm_loader NOLOGIN;
            END IF;
            loader_role := 'pgwasm_loader';
    END;

    BEGIN
        IF NOT EXISTS (SELECT 1 FROM pg_catalog.pg_roles WHERE rolname = 'pg_wasm_reader') THEN
            CREATE ROLE pg_wasm_reader NOLOGIN;
        END IF;
        reader_role := 'pg_wasm_reader';
    EXCEPTION
        WHEN SQLSTATE '42939' THEN
            IF NOT EXISTS (SELECT 1 FROM pg_catalog.pg_roles WHERE rolname = 'pgwasm_reader') THEN
                CREATE ROLE pgwasm_reader NOLOGIN;
            END IF;
            reader_role := 'pgwasm_reader';
    END;

    EXECUTE format('GRANT USAGE ON SCHEMA pg_wasm TO %I', loader_role);
    EXECUTE format('GRANT USAGE ON SCHEMA pg_wasm TO %I', reader_role);

    EXECUTE format(
        'GRANT SELECT ON TABLE pg_wasm.dependencies, pg_wasm.exports, pg_wasm.modules, pg_wasm.wit_types TO %I',
        reader_role
    );

    EXECUTE format(
        'GRANT DELETE, INSERT, SELECT, UPDATE ON TABLE pg_wasm.dependencies, pg_wasm.exports, pg_wasm.modules, pg_wasm.wit_types TO %I',
        loader_role
    );
END
$$;
