//! String proximity lookup function.

/// Finds a near match of `str_to_check` in `strings`.
pub fn find_near(str_to_check: &String, strings: &Vec<String>) -> Option<String> {
    let mut result = None;
    let mut best_distance = str_to_check.len();
    for string in strings {
        let distance = levenshtein_distance(&string, str_to_check);
        if distance < best_distance {
            best_distance = distance;
            if distance < 3 {
                result = Some(string.clone());
            }
        }
    }
    result
}

/// Returns the Levensthein distance between `string1` and `string2`.
fn levenshtein_distance(string1: &String, string2: &String) -> usize {
    fn distance(i: usize, j: usize, d: &Vec<Vec<usize>>, string1: &String, string2: &String) -> usize {
        match (i, j) {
            (i, 0) => i,
            (0, j) => j,
            (i, j) => {
                let delta =
                    if string1.chars().nth(i - 1) == string2.chars().nth(j - 1) {
                        0
                    }
                    else {
                        1
                    };
                *[
                    d[i - 1][j] + 1,
                    d[i][j - 1] + 1,
                    d[i - 1][j - 1] + delta
                ].iter().min().unwrap()
            },
        }
    }

    let mut d = vec![];
    for i in 0 .. string1.len() + 1 {
        d.push(vec![]);
        for j in 0 .. string2.len() + 1 {
            let dist = distance(i, j, &d, string1, string2);
            d[i].push(dist);
        }
    }
    d[string1.len()][string2.len()]
}
