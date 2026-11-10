use super::*;
use serde_json::{Value, json};
use umran_sim::notation::{self, RuleStatus};
use umran_sim::{ENGINE_REVISION, LanguageDesign};

fn found(preset: &str, region: usize) -> String {
    let design = LanguageDesign::preset(preset, 1).unwrap();
    json!({
        "kind": "found", "naming": {"kind": "people"}, "design": design,
        "seed": 3, "power": 0.5, "openness": 0.5, "region": region
    })
    .to_string()
}

/// A founder, a daughter founded with its speech, and a stranger in trade
/// with the founder. After thirty generations the author gives the founder
/// a law and has the stranger take up the founder's language, so the
/// stranger's own falls silent; five more generations pass.
fn bench() -> Bench {
    let mut w = Bench::new(5, "medium").unwrap();
    let map = &w.chronicle.latest().map;
    let anchor = map
        .regions
        .iter()
        .enumerate()
        .filter(|(_, r)| r.terrain.is_land())
        .max_by_key(|&(r, _)| map.walking_row(r, 800.0).len())
        .unwrap()
        .0;
    let sites: Vec<usize> = serde_json::from_str(&w.founding_sites(anchor, 3).unwrap()).unwrap();
    w.act(&found("familiar", anchor)).unwrap();
    w.act(
        &json!({
            "kind": "found-related", "source": 0, "region": sites[1],
            "naming": {"kind": "people"}
        })
        .to_string(),
    )
    .unwrap();
    w.act(&found("polynesian", sites[2])).unwrap();
    w.act(r#"{"kind":"connect","a":0,"b":2,"intensity":0.6,"contact":"trade"}"#)
        .unwrap();
    w.act(r#"{"kind":"run","generations":30}"#).unwrap();
    w.act(r#"{"kind":"law","variety":0,"law":"spirantization"}"#)
        .unwrap();
    w.act(r#"{"kind":"shift","community":2,"toward":0}"#)
        .unwrap();
    w.act(r#"{"kind":"run","generations":5}"#).unwrap();
    w
}

fn take(w: &mut Bench, what: Value) -> Value {
    let latest = w.latest();
    serde_json::from_str(&w.take(latest, &what.to_string()).unwrap()).unwrap()
}

fn text(taken: &Value) -> &str {
    taken["text"].as_str().unwrap()
}

/// A CSV file as rows of cells, undoing the quoting.
fn cells(csv: &str) -> Vec<Vec<String>> {
    let mut rows = vec![vec![String::new()]];
    let mut quoted = false;
    let mut chars = csv.chars().peekable();
    while let Some(c) = chars.next() {
        let row = rows.last_mut().unwrap();
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                chars.next();
                row.last_mut().unwrap().push('"');
            }
            '"' => quoted = !quoted,
            ',' if !quoted => row.push(String::new()),
            '\n' if !quoted => rows.push(vec![String::new()]),
            c => row.last_mut().unwrap().push(c),
        }
    }
    assert_eq!(rows.pop().unwrap(), [""], "the file ends with a line break");
    rows
}

fn overview(w: &mut Bench) -> Value {
    let latest = w.latest();
    serde_json::from_str(&w.overview(latest).unwrap()).unwrap()
}

#[test]
fn a_sheet_quotes_what_would_break_a_row() {
    let mut sheet = Sheet::default();
    sheet.row(["plain", "a, b", "say \"so\"", "two\nlines", ""]);
    let csv = sheet.text();
    assert_eq!(csv, "plain,\"a, b\",\"say \"\"so\"\"\",\"two\nlines\",\n");
    assert_eq!(
        cells(&csv),
        [["plain", "a, b", "say \"so\"", "two\nlines", ""]]
    );
}

#[test]
fn the_lexicon_sheet_has_a_row_for_every_current_word() {
    let mut w = bench();
    let latest = w.latest();
    let taken = take(&mut w, json!({"what": "lexicon", "variety": 0}));
    assert_eq!(taken["extension"], "csv");
    let name = taken["name"].as_str().unwrap();
    assert!(
        name.ends_with(&format!("lexicon year {}", latest * 25)),
        "{name}"
    );
    let rows = cells(text(&taken));
    assert_eq!(
        rows[0],
        [
            "meaning", "word", "said", "ipa", "class", "field", "origin", "from", "since",
            "standing", "id"
        ]
    );
    let lexicon: Vec<Value> = serde_json::from_str(&w.lexicon(latest, 0).unwrap()).unwrap();
    let mut sheet = rows[1..].iter();
    let mut competing = 0;
    for row in &lexicon {
        let line = sheet.next().unwrap();
        assert_eq!(line.len(), 11);
        assert_eq!(line[0], row["gloss"].as_str().unwrap());
        assert_eq!(line[1], row["spelled"].as_str().unwrap());
        assert_eq!(line[3], row["ipa"].as_str().unwrap());
        assert_eq!(
            line[8],
            (row["origin"]["generation"].as_u64().unwrap() * 25).to_string()
        );
        assert_eq!(line[9], "main");
        assert_eq!(line[10], row["concept"].as_str().unwrap());
        for form in row["competingForms"].as_array().unwrap() {
            let line = sheet.next().unwrap();
            assert_eq!(line[0], row["gloss"].as_str().unwrap());
            assert_eq!(line[1], form["spelled"].as_str().unwrap());
            assert_eq!(line[9], "competing");
            competing += 1;
        }
    }
    assert!(sheet.next().is_none());
    assert!(competing > 0, "the run leaves some words competing");
    // The author's law changed words the trade had already lent.
    assert!(
        rows.iter()
            .any(|row| row[6] == "borrowed" && !row[7].is_empty())
    );
}

#[test]
fn the_names_sheet_has_a_row_for_every_name_in_the_quarry() {
    let mut w = bench();
    let latest = w.latest();
    let rows = cells(text(&take(&mut w, json!({"what": "names", "variety": 0}))));
    assert_eq!(
        rows[0],
        ["kind", "name", "ipa", "meaning", "since", "origin"]
    );
    let quarry: Value = serde_json::from_str(&w.name_quarry(latest, 0).unwrap()).unwrap();
    let count = |key: &str| quarry[key].as_array().unwrap().len();
    assert_eq!(
        rows.len() - 1,
        count("given") + count("peoples") + count("places") + count("states") + count("faiths")
    );
    assert_eq!(rows[1][0], "given name");
    assert_eq!(rows[1][1], quarry["given"][0]["name"].as_str().unwrap());
    assert!(rows.iter().any(|row| row[0] == "people"));
}

#[test]
fn the_law_ledger_is_dated_and_reads_back_in_the_law_form() {
    let mut w = bench();
    let latest = w.latest();
    let taken = take(&mut w, json!({"what": "laws", "variety": 0}));
    assert_eq!(taken["extension"], "txt");
    let ledger = text(&taken);
    let overview = overview(&mut w);
    let laws = overview["varieties"][0]["laws"].as_array().unwrap();
    assert!(!laws.is_empty());
    let dated: Vec<&str> = ledger
        .lines()
        .filter(|line| line.starts_with("# Year "))
        .collect();
    assert_eq!(dated.len(), laws.len());
    // Earliest first, each under its own year and label.
    for (line, law) in dated.iter().zip(laws) {
        assert_eq!(
            *line,
            format!(
                "# Year {}: {}",
                law["generation"].as_u64().unwrap() * 25,
                law["label"].as_str().unwrap()
            )
        );
    }
    assert!(ledger.contains("# The author's decision "));
    // Everything that is not a comment is a rule the law form reads.
    let report = notation::parse("ledger", ledger);
    assert!(
        report
            .rules
            .iter()
            .any(|rule| rule.status == RuleStatus::Ok)
    );
    for rule in &report.rules {
        assert!(
            matches!(
                rule.status,
                RuleStatus::Ok | RuleStatus::Empty | RuleStatus::DuplicateStress
            ),
            "{}: {}",
            rule.source,
            rule.message
        );
    }

    // The daughter's ledger opens with what it inherited.
    let daughter = take(&mut w, json!({"what": "laws", "variety": 1}));
    let inherited = text(&daughter).matches("# Inherited from ").count();
    let own = text(&daughter).matches("# Year ").count();
    assert!(own >= inherited);

    let data = take(&mut w, json!({"what": "laws-data", "variety": 0}));
    assert_eq!(data["extension"], "json");
    let data: Value = serde_json::from_str(text(&data)).unwrap();
    assert_eq!(data["revision"], ENGINE_REVISION);
    assert_eq!(data["generation"], latest);
    assert_eq!(data["yearsPerGeneration"], 25);
    let recorded = data["laws"].as_array().unwrap();
    assert_eq!(recorded.len(), laws.len());
    assert!(recorded.iter().any(|law| {
        law["words"]
            .as_array()
            .unwrap()
            .iter()
            .any(|word| word["before"]["ipa"] != word["after"]["ipa"])
    }));
}

#[test]
fn the_phrasebook_sets_each_gloss_under_its_word() {
    let mut w = bench();
    let latest = w.latest();
    let taken = take(&mut w, json!({"what": "phrasebook", "variety": 0}));
    assert_eq!(taken["extension"], "txt");
    let phrases: Vec<Value> = serde_json::from_str(&w.phrasebook(latest, 0).unwrap()).unwrap();
    assert!(!phrases.is_empty());
    let lines: Vec<&str> = text(&taken).lines().collect();
    assert!(lines[0].starts_with("The phrasebook of "));
    for (number, phrase) in phrases.iter().enumerate() {
        let tag = format!("({})", number + 1);
        let at = lines
            .iter()
            .position(|line| line.starts_with(&tag))
            .unwrap();
        // Where each word starts, counting letters and not the marks on them.
        let starts = |line: &str| -> Vec<usize> {
            let mut starts = Vec::new();
            let (mut column, mut inside) = (0, false);
            for c in line.chars().skip(tag.chars().count()) {
                if c.is_whitespace() == inside {
                    inside = !inside;
                    if inside {
                        starts.push(column);
                    }
                }
                column += usize::from(crate::phrasebook::wide(c));
            }
            starts
        };
        let words = phrase["text"].as_str().unwrap().split_whitespace().count();
        assert_eq!(starts(lines[at]).len(), words);
        assert_eq!(starts(lines[at]), starts(lines[at + 1]));
        assert_eq!(starts(lines[at]), starts(lines[at + 2]));
        assert_eq!(
            lines[at + 3].trim(),
            format!("‘{}’", phrase["translation"].as_str().unwrap())
        );
    }
}

#[test]
fn the_word_sheet_takes_silent_languages_and_gives_each_root_its_first_form() {
    let mut w = bench();
    let overview = overview(&mut w);
    let varieties = overview["varieties"].as_array().unwrap();
    assert_eq!(varieties[2]["spoken"], false);
    let silent = varieties[2]["silentSince"].as_u64().unwrap();
    let taken = take(
        &mut w,
        json!({"what": "words", "varieties": [0, 1, 2], "concepts": ["water", "fire", "hand"]}),
    );
    let rows = cells(text(&taken));
    let name = |id: usize| varieties[id]["name"].as_str().unwrap().to_string();
    let stranger = format!("{} (silent since year {})", name(2), silent * 25);
    assert_eq!(
        rows[0],
        [
            "meaning".to_string(),
            name(0),
            format!("{} (IPA)", name(0)),
            name(1),
            format!("{} (IPA)", name(1)),
            stranger.clone(),
            format!("{stranger} (IPA)"),
            format!("{} root", name(0)),
            format!("{} root", name(1)),
            format!("{stranger} root"),
            format!("{} origin", name(0)),
            format!("{} origin", name(1)),
            format!("{stranger} origin"),
            "first forms".to_string(),
        ]
    );
    assert_eq!(rows.len(), 4);
    for row in &rows[1..] {
        // The stranger's word stands as it was when its speakers left it.
        assert!(!row[5].is_empty(), "{row:?}");
        // Each root in the row is listed once, under its number.
        let mut roots: Vec<&str> = row[7..10].iter().map(String::as_str).collect();
        roots.sort_unstable();
        roots.dedup();
        let firsts: Vec<&str> = row[13].split("; ").collect();
        assert_eq!(firsts.len(), roots.len(), "{row:?}");
        for root in roots {
            assert!(
                firsts
                    .iter()
                    .any(|first| first.starts_with(&format!("{root}: "))),
                "{row:?}"
            );
        }
    }
    // The founder and its daughter share at least one root here.
    assert!(rows[1..].iter().any(|row| row[7] == row[8]));
}

/// A Newick tree as each tip's name and its distance from the root's start.
fn tips(tree: &str) -> Vec<(String, u32)> {
    fn node(chars: &[char], at: &mut usize, above: u32, out: &mut Vec<(String, u32)>) {
        let mut inner = Vec::new();
        let mut name = String::new();
        if chars[*at] == '(' {
            *at += 1;
            loop {
                inner.push(*at);
                // Skip the child to learn this node's own length first.
                let mut depth = 0;
                let mut quoted = false;
                while quoted || depth > 0 || !matches!(chars[*at], ',' | ')') {
                    match chars[*at] {
                        '\'' => quoted = !quoted,
                        '(' if !quoted => depth += 1,
                        ')' if !quoted => depth -= 1,
                        _ => {}
                    }
                    *at += 1;
                }
                *at += 1;
                if chars[*at - 1] == ')' {
                    break;
                }
            }
        } else if chars[*at] == '\'' {
            *at += 1;
            loop {
                match (chars[*at], chars.get(*at + 1)) {
                    ('\'', Some('\'')) => {
                        name.push('\'');
                        *at += 2;
                    }
                    ('\'', _) => {
                        *at += 1;
                        break;
                    }
                    (c, _) => {
                        name.push(c);
                        *at += 1;
                    }
                }
            }
        } else {
            while chars[*at] != ':' {
                name.push(chars[*at]);
                *at += 1;
            }
        }
        let mut length = String::new();
        if chars.get(*at) == Some(&':') {
            *at += 1;
            while chars.get(*at).is_some_and(char::is_ascii_digit) {
                length.push(chars[*at]);
                *at += 1;
            }
        }
        let here = above + length.parse::<u32>().unwrap_or(0);
        if inner.is_empty() {
            out.push((name, here));
        }
        for mut child in inner {
            node(chars, &mut child, here, out);
        }
    }
    let chars: Vec<char> = tree.trim_end().chars().collect();
    assert_eq!(chars.last(), Some(&';'));
    let mut out = Vec::new();
    node(&chars, &mut 0, 0, &mut out);
    out
}

#[test]
fn the_tree_gives_every_language_a_tip_as_far_from_the_root_as_it_lived() {
    let mut w = bench();
    let latest = w.latest();
    let overview = overview(&mut w);
    let varieties = overview["varieties"].as_array().unwrap();
    let taken = take(&mut w, json!({"what": "tree", "family": null}));
    assert_eq!(taken["extension"], "nwk");
    let trees: Vec<&str> = text(&taken).lines().collect();
    let roots: Vec<&Value> = varieties.iter().filter(|v| v["parent"].is_null()).collect();
    assert_eq!(trees.len(), roots.len());
    let mut seen = 0;
    for (tree, root) in trees.iter().zip(&roots) {
        for (name, years) in tips(tree) {
            let v = varieties
                .iter()
                .find(|v| v["name"] == name.as_str() && v["family"] == root["id"])
                .unwrap_or_else(|| panic!("{name} is in {tree}"));
            let ended = v["silentSince"].as_u64().unwrap_or(u64::from(latest));
            let lived = (ended - root["born"].as_u64().unwrap()) * 25;
            assert_eq!(u64::from(years), lived, "{name} in {tree}");
            seen += 1;
        }
    }
    assert_eq!(seen, varieties.len());
    // The stranger's language is a tip that stops short of this reading.
    assert_eq!(varieties[2]["spoken"], false);

    let family = take(&mut w, json!({"what": "tree", "family": 0}));
    assert_eq!(text(&family), format!("{}\n", trees[0]));
    let kin = varieties.iter().filter(|v| v["family"] == 0).count();
    assert!(kin >= 2);
    assert_eq!(tips(trees[0]).len(), kin);
    let refused = w.take(latest, r#"{"what":"tree","family":1}"#).unwrap_err();
    assert_eq!(refused, "A family is named by its first language.");
}

#[test]
fn a_tree_label_is_quoted_unless_it_is_letters_alone() {
    assert_eq!(label("Aravi"), "Aravi");
    assert_eq!(label("Áraví"), "Áraví");
    assert_eq!(label("Old Aravi"), "'Old Aravi'");
    assert_eq!(label("Ta'ino (2)"), "'Ta''ino (2)'");
}

#[test]
fn a_request_for_nothing_known_is_refused_by_name() {
    let mut w = bench();
    let latest = w.latest();
    let refused = w.take(latest, r#"{"what":"atlas"}"#).unwrap_err();
    assert!(
        refused.starts_with("That is not something to take out: "),
        "{refused}"
    );
    let refused = w
        .take(latest, r#"{"what":"lexicon","variety":99}"#)
        .unwrap_err();
    assert_eq!(refused, "No such language variety.");
}
