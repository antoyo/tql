/*
 * Copyright (c) 2017-2018 Boucher, Antoni <bouanto@zoho.com>
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy of
 * this software and associated documentation files (the "Software"), to deal in
 * the Software without restriction, including without limitation the rights to
 * use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
 * the Software, and to permit persons to whom the Software is furnished to do so,
 * subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
 * FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
 * COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
 * IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
 * CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

//! String proximity lookup function.

use std::cmp;

use quote::ToTokens;

/// Variadic minimum macro. It returns the minimum of its arguments.
macro_rules! min {
    ( $e:expr ) => {
        $e
    };
    ( $e:expr, $( $rest:expr ),* ) => {
        cmp::min($e, min!($( $rest ),*))
    };
}

/// Finds a near match of `str_to_check` in `strings`.
//#[allow(needless_lifetimes)]
pub fn find_near<'a, T>(str_to_check: &str, strings: T) -> Option<&'a str>
    where T: Iterator<Item = &'a str>
{
    let mut result = None;
    let mut best_distance = str_to_check.len();
    for string in strings {
        let distance = levenshtein_distance(&string, str_to_check);
        if distance < best_distance {
            best_distance = distance;
            if distance < 3 {
                result = Some(string);
            }
        }
    }
    result
}

/// Returns the Levensthein distance between `string1` and `string2`.
//#[allow(needless_range_loop)]
fn levenshtein_distance(string1: &str, string2: &str) -> usize {
    fn distance(i: usize, j: usize, d: &[Vec<usize>], string1: &str, string2: &str) -> usize {
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
                min!( d[i - 1][j] + 1
                    , d[i][j - 1] + 1
                    , d[i - 1][j - 1] + delta
                    )
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

/// Returns " was" if count equals 1, "s were" otherwise.
pub fn plural_verb<'a>(count: usize) -> &'a str {
    if count == 1 {
        " was"
    }
    else {
        "s were"
    }
}

/// Convert a syn object to a string.
pub fn token_to_string<T: ToTokens>(token: &T) -> String {
    (quote! { #token }).to_string()
}
