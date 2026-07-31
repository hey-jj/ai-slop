#![no_main]
//! The extractor and full pipeline must never panic on arbitrary bytes, and
//! every emitted span must hold the emit-time invariant, which analyze
//! enforces internally.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    for profile in [
        ai_slop::Profile::PublicBugReport,
        ai_slop::Profile::CommitMessage,
        ai_slop::Profile::CargoMetadata,
    ] {
        let config = ai_slop::Config::new(profile);
        let _ = ai_slop::analyze(data, &config);
    }
});
