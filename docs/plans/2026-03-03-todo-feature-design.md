# Todo Feature Design

## Summary

Add a task queue (todo list) to pomitik. Tasks are managed via CLI subcommands and displayed alongside the timer in a sidebar layout. The top pending task is the "current task" shown prominently above the timer. Users interact with the list during timer sessions via tab-focus switching and keyboard controls.

## Data Model

**File:** `~/.local/share/pomitik/todos.json`

```rust
struct Todo {
    id: u32,
    text: String,
    done: bool,
    created_at: DateTime<Local>,
    completed_at: Option<DateTime<Local>>,
}

struct TodoList {
    next_id: u32,
    items: Vec<Todo>,  // position 0 = current task
}
```

- IDs are auto-incrementing and never reused (stable references for bots).
- Array order defines queue priority.
- Single JSON file, read-modify-write on each operation.

## CLI Commands

```
tik todo                         # Alias for 'tik todo list'
tik todo add "Task text"         # Append task to end of pending queue
tik todo list [--json]           # Show all todos; --json for machine-readable output
tik todo done <id>               # Mark task as completed
tik todo remove <id>             # Delete task entirely
tik todo move <id> <position>    # Move task to position (1-based)
tik todo edit <id> "new text"    # Edit task text
tik todo clear                   # Remove all completed tasks
```

**List output format:**
```
Tasks:
  1. [ ] Review PR #42        (#1)
  2. [ ] Write unit tests      (#3)
  3. [x] Fix login bug         (#2)
```

Position on left, stable ID in parens on right. `--json` outputs raw TodoList JSON.

## Timer UI: Sidebar Layout

When pending todos exist, the timer renders in a two-column layout:

```
         > Review PR #42             |
                                     |  Tasks:
           [Round 2/4]               |  > Review PR #42
             24:35                   |    Write tests
    [xxxxxxxxxxxx...............]    |    Update docs
          12:30 elapsed              |  v Fix login bug
                                     |
  [space] pause [s] skip [tab] tasks |
```

- Current task (first pending item) shown above round info.
- Sidebar: ~30 chars wide, vertical separator, full task list.
- If no pending todos, falls back to classic centered layout (no sidebar).

## Focus Modes

**Timer focus (default):**
- All existing keys work: space, s, a, d, x, Ctrl+C.
- Hint bar includes `[tab] tasks`.

**Todo focus (after pressing tab):**
- Arrow keys (up/down) navigate selection highlight.
- Enter marks selected task as done.
- Shift+up/down reorders tasks.
- Hint bar: `[tab] timer [up/dn] select [enter] done [S-up/dn] move`
- Timer continues running; only input routing changes.

## Architecture

**New file:** `src/todo.rs`
- `TodoList` struct: load(), save(), add(), remove(), done(), move_to(), edit(), clear()
- JSON serde, file path via `dirs::data_dir()`

**Modified files:**

- `main.rs`: Add `Todo` subcommand enum (Add, List, Done, Remove, Move, Edit, Clear). Route to `todo.rs`.
- `timer.rs`: Accept optional `TodoList`. Add `todo_focus` watch channel. Handle tab, arrows, enter, shift+arrows in input thread.
- `render.rs`: Extend `DrawParams` with todo state (items, selected index, focus mode). Two-column layout when todos present.
- `session.rs`: Load TodoList at session start, pass through to timer.

**Unchanged:** `config.rs`, `duration.rs`, `log.rs`, `notify.rs`.
