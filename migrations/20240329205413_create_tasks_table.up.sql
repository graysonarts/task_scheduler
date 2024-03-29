CREATE TABLE IF NOT EXISTS tasks (
    id Uuid PRIMARY KEY,
    kind task_type NOT NULL,
    status task_status_type NOT NULL DEFAULT 'Pending',
    process_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
) ;