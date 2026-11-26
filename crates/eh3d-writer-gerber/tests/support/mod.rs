//! The synthetic files of the repository and a way to damage them that is the same every time.

use std::path::Path;

/// The files of `tests/golden` and `tests/corpus` of the repository with one of these extensions.
pub fn sources(extensions: &[&str]) -> Vec<(String, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests");
    let mut folders = vec![root];
    let mut out = Vec::new();
    while let Some(folder) = folders.pop() {
        for entry in std::fs::read_dir(folder).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                folders.push(path);
                continue;
            }
            let extension = path
                .extension()
                .map(|e| e.to_string_lossy().to_ascii_lowercase())
                .unwrap_or_default();
            if extensions.contains(&extension.as_str()) {
                let bytes = std::fs::read(&path).unwrap();
                out.push((
                    path.display().to_string(),
                    String::from_utf8_lossy(&bytes).replace("\r\n", "\n"),
                ));
            }
        }
    }
    out.sort();
    out
}

pub struct Random(pub u64);

impl Random {
    pub fn below(&mut self, n: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % n.max(1) as u64) as usize
    }
}

/// The text with a few lines lost, repeated or moved and a few characters changed.
pub fn damaged(text: &str, random: &mut Random) -> String {
    let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
    for _ in 0..1 + random.below(4) {
        if lines.is_empty() {
            break;
        }
        let at = random.below(lines.len());
        match random.below(4) {
            0 => {
                lines.remove(at);
            }
            1 => {
                let line = lines[at].clone();
                lines.insert(random.below(lines.len()), line);
            }
            _ => {
                let mut chars: Vec<char> = lines[at].chars().collect();
                let with = [
                    '0', '1', '9', '-', '.', ',', 'X', 'Y', 'T', 'G', 'M', 'A', 'I', 'D', '*', '%',
                ];
                let new = with[random.below(with.len())];
                if chars.is_empty() || random.below(3) == 0 {
                    chars.insert(random.below(chars.len() + 1), new);
                } else {
                    let place = random.below(chars.len());
                    chars[place] = new;
                }
                lines[at] = chars.into_iter().collect();
            }
        }
    }
    lines.join("\n") + "\n"
}
