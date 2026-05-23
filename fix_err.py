import re

path = r'C:\Users\shada\.kimi_openclaw\workspace\openclaw-swarm\src\execution_loop.rs'
with open(path, 'r') as f:
    content = f.read()

old = '''            // Log to error journal
            let error_journal = ErrorJournal::new(&format!("{}/error_journal.db", workspace))?;
            let _ = error_journal.log_error(
                &task_id,
                &persona_id,
                &e.to_string(),
                &classify_error(&e.to_string()),
            );'''

new = '''            // Log to error journal
            let error_journal = ErrorJournal::new(&format!("{}/error_journal.db", workspace))?;
            let error_log = ErrorLog {
                id: uuid::Uuid::new_v4(),
                task_id: task_id.clone(),
                persona_id: persona_id.clone(),
                error_message: e.to_string(),
                error_type: classify_error(&e.to_string()),
                file_path: None,
                line_number: None,
                root_cause: None,
                solution: None,
                same_symptom_different_cause: false,
                occurred_at: chrono::Utc::now(),
            };
            let _ = error_journal.log_error(&error_log);'''

if old in content:
    content = content.replace(old, new)
    with open(path, 'w') as f:
        f.write(content)
    print('Fixed error journal call')
else:
    print('Pattern not found')
    for i, line in enumerate(content.split('\n')):
        if 'error_journal.log_error' in line:
            print(f'Line {i+1}: {line.strip()}')
