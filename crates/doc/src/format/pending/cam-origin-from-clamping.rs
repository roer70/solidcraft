//! A setup's work origin taken from the setup whose clamping it stays in
//! (`manufacture.setups[*].origin`: `{"type": "clamping"}`, with `same_clamping`). Older designs
//! need no transformation: they have no such origin. The last release asks for an update instead
//! of opening a design that has it, whose origin it could not read.
