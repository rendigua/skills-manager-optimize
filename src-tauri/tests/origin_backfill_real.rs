use app_lib::core::{central_repo, origin_backfill, source_resolver};

#[test]
#[ignore = "mutates the real central skills repo"]
fn apply_safe_backfill_to_central_repo() {
    let root = std::env::var("SKILLS_BACKFILL_ROOT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| central_repo::skills_dir());
    let limit = 30;

    let plan = origin_backfill::plan_origin_backfill(&root, limit, |query, limit, api_key| {
        source_resolver::resolve_source_candidates(query, limit, api_key)
    })
    .expect("plan backfill");

    let selected_paths = plan
        .entries
        .iter()
        .filter(|entry| entry.action == origin_backfill::BackfillAction::ApplyCandidate)
        .map(|entry| entry.path.clone())
        .collect::<Vec<_>>();

    let result = origin_backfill::apply_origin_backfill(&root, &selected_paths, limit, |query, limit, api_key| {
        source_resolver::resolve_source_candidates(query, limit, api_key)
    })
    .expect("apply backfill");

    println!(
        "root={} planned={} applied={} skipped={} review_needed={}",
        plan.root,
        plan.entries.len(),
        result.applied,
        result.skipped,
        result.review_needed
    );

    for entry in plan.entries.iter().filter(|entry| entry.action == origin_backfill::BackfillAction::ApplyCandidate).take(10) {
        println!("applied candidate: {} -> {}", entry.name, entry.top_candidate.as_ref().map(|c| c.source_ref_resolved.as_str()).unwrap_or("<none>"));
    }
}
