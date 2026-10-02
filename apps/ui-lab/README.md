# VoxProof UI Lab

A standalone egui experiment in review ergonomics and front-of-house navigation
(Home, Course, "what it learned", Intro, Review). It uses invented demo data and
simulated audio. It never reads or writes VoxProof sessions, ledgers,
persistence, or reviewed output, and it is not product behavior or validation.
It depends on the VoxProof crate only for the SRT parser and the
non-authoritative `experimental_retrieval` near-sound matcher, so remembered
spellings can also match differently spelled but similar-sounding text.

```bash
CARGO_TARGET_DIR=../../target cargo run --manifest-path apps/ui-lab/Cargo.toml
```

Developer aid (replays commands, saves the last frame as PPM, then exits):

```bash
VOXLAB_SCRIPT="start;select:1;accept;learn:course;go:memory" VOXLAB_SHOT=/tmp/shot.ppm \
  ./target/debug/voxproof-ui-lab
```

Commands: `go:home|course|memory|intro`, `start`, `select:N`, `accept`, `keep`,
`unsure`, `edit`, `group`, `groupaccept`, `learn:course|me|skip`, `meaning`,
`teacher`, `dark`, `finish`, `receipt`, `undo`, `play`, `unsure_review`.
