# Agent Instructions

- When starting any local dev server, backend server, watcher, or long-running process, stop it before finishing the task unless the user explicitly asks to keep it running.
- If a server was already running before the task and you start another process because of a port conflict, clean up only the processes you started unless the user asks you to stop the pre-existing server too.
