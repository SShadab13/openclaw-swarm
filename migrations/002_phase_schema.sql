-- Swarm Execution Bridge — Phase Schema
-- PRD: docs/PRD_EXECUTION_BRIDGE.md
-- Date: May 9, 2026

-- Phases table: tracks decomposition of tasks into executable chunks
CREATE TABLE IF NOT EXISTS phases (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    phase_number INTEGER NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL CHECK(status IN ('pending', 'running', 'done', 'failed', 'retrying')),
    assigned_agent TEXT,
    files_expected TEXT, -- JSON array of file paths
    files_created TEXT,  -- JSON array of actual file paths
    started_at TEXT,
    completed_at TEXT,
    error_output TEXT,
    retry_count INTEGER DEFAULT 0,
    handoff_letter TEXT,
    FOREIGN KEY (task_id) REFERENCES tasks(id)
);

-- Phase dependencies: which phases must complete before others
CREATE TABLE IF NOT EXISTS phase_dependencies (
    phase_id TEXT NOT NULL,
    depends_on_phase_id TEXT NOT NULL,
    PRIMARY KEY (phase_id, depends_on_phase_id),
    FOREIGN KEY (phase_id) REFERENCES phases(id),
    FOREIGN KEY (depends_on_phase_id) REFERENCES phases(id)
);

-- Task decomposition plans: the original breakdown
CREATE TABLE IF NOT EXISTS decomposition_plans (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    total_phases INTEGER NOT NULL,
    plan_json TEXT NOT NULL, -- Full decomposition JSON
    created_at TEXT NOT NULL,
    FOREIGN KEY (task_id) REFERENCES tasks(id)
);

-- Phase execution log: detailed history of each attempt
CREATE TABLE IF NOT EXISTS phase_execution_log (
    id TEXT PRIMARY KEY,
    phase_id TEXT NOT NULL,
    attempt_number INTEGER NOT NULL,
    started_at TEXT,
    completed_at TEXT,
    status TEXT CHECK(status IN ('success', 'timeout', 'error')),
    files_written TEXT, -- JSON array
    output_summary TEXT,
    error_text TEXT,
    FOREIGN KEY (phase_id) REFERENCES phases(id)
);

-- Index for fast lookups
CREATE INDEX IF NOT EXISTS idx_phases_task ON phases(task_id);
CREATE INDEX IF NOT EXISTS idx_phases_status ON phases(status);
CREATE INDEX IF NOT EXISTS idx_phases_number ON phases(task_id, phase_number);
CREATE INDEX IF NOT EXISTS idx_deps_phase ON phase_dependencies(phase_id);
CREATE INDEX IF NOT EXISTS idx_deps_depends ON phase_dependencies(depends_on_phase_id);
