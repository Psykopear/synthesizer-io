# Audio callback
- run the graph to generate the samples
- read messages from the engine and update the graph
- send messages to the engine

# Engine
The engine runs on a separate thread.
The thread does something like:
```rust
std::thread::spawn(move || {
  engine.run_step();
  std::thread::sleep(Duration::from_millis(1));
})
```

We want to run the engine fast, but not clog an entire CPU, we don't need sample-level accuracy here.
TODO: Make some calculations to justify this, but look at it like SuperCollider's kr (control rate)

The step:
- Read all messages coming from the audio thread
- Extract latest timestamp, proxy it to the UI (see next section)
- Extract the events that needs to be sent to the worker, send them
- Update engine's tempo structure with the last timestamp received from the audio thread

# UI
The ui updates periodically, something like 60fps should be enough.
At each update, it reads messages coming from the engine, and
