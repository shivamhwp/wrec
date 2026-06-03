use crate::protocol::{AgentError, TargetSelector};
use wrec_backend::{capture_kind_arg, resolve_target};
use wrec_core::{CaptureSourceKind, CaptureTarget, RecorderSettings};

pub(crate) fn resolve_record_target(
    targets: &[CaptureTarget],
    kind: CaptureSourceKind,
    selector: Option<&TargetSelector>,
    saved_id: Option<u64>,
) -> Result<CaptureTarget, AgentError> {
    match selector {
        Some(TargetSelector::Id { kind, id }) => {
            resolve_target(targets, *kind, Some(*id), None).map_err(target_error)
        }
        Some(TargetSelector::Name { kind, query }) => {
            let candidates = targets
                .iter()
                .filter(|target| kind.map_or(true, |kind| target.kind == kind))
                .collect::<Vec<_>>();
            resolve_by_name(candidates, query, "target")
        }
        Some(TargetSelector::App { query }) => {
            let candidates = targets
                .iter()
                .filter(|target| target.kind == CaptureSourceKind::Window)
                .collect::<Vec<_>>();
            resolve_by_app(candidates, query)
        }
        None => resolve_target(targets, kind, None, saved_id).map_err(target_error),
    }
}

pub(crate) fn settings_for_target(
    mut settings: RecorderSettings,
    target: &CaptureTarget,
) -> RecorderSettings {
    settings.source = target.kind;
    settings
}

fn resolve_by_name(
    candidates: Vec<&CaptureTarget>,
    query: &str,
    label: &str,
) -> Result<CaptureTarget, AgentError> {
    let query = normalized(query);
    if query.is_empty() {
        return Err(AgentError {
            code: "empty_target_query".into(),
            message: format!("{label} query cannot be empty"),
            recoverable: true,
            next: "Pass a non-empty target name or use `wrec targets --json` to choose an id."
                .into(),
        });
    }

    for predicate in [MatchKind::Exact, MatchKind::Prefix, MatchKind::Contains] {
        let matches = candidates
            .iter()
            .copied()
            .filter(|target| match predicate {
                MatchKind::Exact => normalized(&target.name) == query,
                MatchKind::Prefix => normalized(&target.name).starts_with(&query),
                MatchKind::Contains => normalized(&target.name).contains(&query),
            })
            .collect::<Vec<_>>();
        if !matches.is_empty() {
            return unique_match(matches, label, &query);
        }
    }

    Err(AgentError {
        code: "target_not_found".into(),
        message: format!("no {label} matches `{query}`"),
        recoverable: true,
        next: "Run `wrec targets --json` and pass `--target kind:id` for an exact target.".into(),
    })
}

fn resolve_by_app(
    candidates: Vec<&CaptureTarget>,
    query: &str,
) -> Result<CaptureTarget, AgentError> {
    let query = normalized(query);
    if query.is_empty() {
        return Err(AgentError {
            code: "empty_app_query".into(),
            message: "app query cannot be empty".into(),
            recoverable: true,
            next: "Pass an app name or use `wrec targets --json` to choose a window id.".into(),
        });
    }

    for predicate in [MatchKind::Exact, MatchKind::Prefix, MatchKind::Contains] {
        let matches = candidates
            .iter()
            .copied()
            .filter(|target| match predicate {
                MatchKind::Exact => normalized(app_name(target)) == query,
                MatchKind::Prefix => normalized(app_name(target)).starts_with(&query),
                MatchKind::Contains => normalized(app_name(target)).contains(&query),
            })
            .collect::<Vec<_>>();
        if !matches.is_empty() {
            return unique_match(matches, "app", &query);
        }
    }

    Err(AgentError {
        code: "app_not_found".into(),
        message: format!("no app matches `{query}`"),
        recoverable: true,
        next: "Run `wrec targets --json` and pass `--target window:id` for an exact window.".into(),
    })
}

enum MatchKind {
    Exact,
    Prefix,
    Contains,
}

fn unique_match(
    matches: Vec<&CaptureTarget>,
    label: &str,
    query: &str,
) -> Result<CaptureTarget, AgentError> {
    match matches.as_slice() {
        [target] => Ok((*target).clone()),
        _ => Err(AgentError {
            code: "ambiguous_target".into(),
            message: format!(
                "multiple {label}s match `{query}`: {}",
                matches
                    .iter()
                    .map(|target| {
                        format!(
                            "{}:{} {}",
                            capture_kind_arg(target.kind),
                            target.id,
                            target.name
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            recoverable: true,
            next: "Pass `--target kind:id` to choose one exact target.".into(),
        }),
    }
}

fn normalized(value: &str) -> String {
    value.trim().to_lowercase()
}

fn app_name(target: &CaptureTarget) -> &str {
    target
        .name
        .split_once(" \u{2014} ")
        .map(|(app, _)| app)
        .unwrap_or(&target.name)
}

fn target_error(message: String) -> AgentError {
    AgentError {
        code: "target_not_found".into(),
        message,
        recoverable: true,
        next: "Run `wrec targets --json` and pass one of the listed target ids.".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wrec_core::Resolution;

    #[test]
    fn settings_source_follows_resolved_target() {
        let target = CaptureTarget {
            id: 42,
            name: "Notes - Draft".into(),
            kind: CaptureSourceKind::Window,
        };
        let settings = settings_for_target(RecorderSettings::default(), &target);

        assert_eq!(settings.source, CaptureSourceKind::Window);
    }

    #[test]
    fn selected_resolution_survives_target_resolution() {
        let target = CaptureTarget {
            id: 1,
            name: "Display".into(),
            kind: CaptureSourceKind::Display,
        };
        let settings = RecorderSettings {
            resolution: Resolution::R720p,
            ..RecorderSettings::default()
        };

        assert_eq!(
            settings_for_target(settings, &target).resolution,
            Resolution::R720p
        );
    }
}
