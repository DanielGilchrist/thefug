use strsim::{jaro_winkler, normalized_damerau_levenshtein};

pub fn program(typed: &str, candidate: &str) -> f64 {
    normalized_damerau_levenshtein(typed, candidate)
}

pub fn subcommand(typed: &str, candidate: &str) -> f64 {
    jaro_winkler(typed, candidate).max(normalized_damerau_levenshtein(typed, candidate))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn program_ranks_transposition_above_shared_prefix() {
        assert!(
            program("gti", "git") > program("gti", "gtail"),
            "a one-edit transposition must beat a longer shared-prefix name"
        );
        assert!(program("gti", "git") > program("gti", "gtimeout"));
        assert!(program("gti", "git") > program("gti", "gsettings"));
    }

    #[test]
    fn program_scores_unrelated_names_low() {
        assert!(program("gti", "zellij") < 0.5);
        assert!(program("gti", "docker") < 0.5);
    }

    #[test]
    fn subcommand_handles_short_transpositions() {
        assert!(subcommand("pll", "pull") > 0.5);
        assert!(subcommand("sttaus", "status") > 0.5);
    }

    #[test]
    fn subcommand_forgives_mangled_typo_with_matching_prefix() {
        assert!(subcommand("puuulllll", "pull") > 0.5);
    }

    #[test]
    fn subcommand_scores_unrelated_tokens_low() {
        assert!(subcommand("pll", "delete-session") < 0.5);
        assert!(subcommand("pll", "checkout") < 0.5);
    }
}
