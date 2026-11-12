//! Saved local player names. These contain no credentials and grant no online identity.

pub fn remember(names: &mut Vec<String>, input: &str) -> Result<String, String> {
    let name = input.trim();
    if !(3..=16).contains(&name.len())
        || !name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
    {
        return Err("Use 3–16 letters, digits or _ for a local profile.".into());
    }
    if let Some(existing) = names.iter().find(|n| n.eq_ignore_ascii_case(name)) {
        return Ok(existing.clone());
    }
    if names.len() >= 64 {
        return Err("The maximum of 64 saved local profiles has been reached.".into());
    }
    names.push(name.to_owned());
    Ok(name.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_canonical_name_and_distinct_profiles() {
        let mut names = Vec::new();
        assert_eq!(
            remember(&mut names, "  Player_One  ").unwrap(),
            "Player_One"
        );
        assert_eq!(remember(&mut names, "player_one").unwrap(), "Player_One");
        assert_eq!(remember(&mut names, "Player_Two").unwrap(), "Player_Two");
        assert_eq!(names, ["Player_One", "Player_Two"]);
    }

    #[test]
    fn rejects_invalid_names_without_mutation() {
        for input in [
            "",
            "ab",
            "abcdefghijklmnopq",
            "bad name",
            "../player",
            "ação",
            "player\nname",
        ] {
            let mut names = vec!["Valid_Name".into()];
            assert!(remember(&mut names, input).is_err(), "{input:?}");
            assert_eq!(names, ["Valid_Name"]);
        }
        assert!(remember(&mut vec![], "abcdefghijklmnop").is_ok());
    }

    #[test]
    fn bounded_list_still_allows_selecting_an_existing_profile() {
        let mut names: Vec<_> = (0..64).map(|n| format!("Player{n}")).collect();
        assert!(remember(&mut names, "NewPlayer").is_err());
        assert_eq!(remember(&mut names, "PLAYER4").unwrap(), "Player4");
        assert_eq!(names.len(), 64);
    }
}
