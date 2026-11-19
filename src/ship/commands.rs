//! One character queue for flight commands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Jump,
    Sneeze,
    Chase,
    Radio,
}

#[derive(Default)]
pub struct CommandReader {
    tail: String,
    idle: f32,
}
impl CommandReader {
    pub fn tick(&mut self, dt: f32) {
        self.idle += dt;
        if self.idle > 5.0 {
            self.tail.clear();
        }
    }
    pub fn clear(&mut self) {
        self.tail.clear();
        self.idle = 0.0;
    }
    pub fn push(&mut self, ch: char) -> Option<Command> {
        self.idle = 0.0;
        let ch = if ch == ',' {
            '.'
        } else {
            ch.to_ascii_lowercase()
        };
        if !ch.is_ascii() {
            self.clear();
            return None;
        }
        self.tail.push(ch);
        if self.tail.len() > 16 {
            self.tail.remove(0);
        }
        for (code, command) in [
            ("3.14", Command::Jump),
            ("achu", Command::Sneeze),
            ("mishmish", Command::Chase),
            ("fm", Command::Radio),
        ] {
            if self.tail.ends_with(code) {
                self.clear();
                return Some(command);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_commands_share_the_queue_and_preserve_spaces() {
        let mut reader = CommandReader::default();
        let commands = "3.14 ACHU mishmish 3,14 FM"
            .chars()
            .filter_map(|c| reader.push(c))
            .collect::<Vec<_>>();
        assert_eq!(
            commands,
            [
                Command::Jump,
                Command::Sneeze,
                Command::Chase,
                Command::Jump,
                Command::Radio
            ]
        );
        for c in "mish".chars() {
            assert_eq!(reader.push(c), None);
        }
        reader.tick(6.0);
        for c in " mish".chars() {
            assert_eq!(reader.push(c), None);
        }
    }
}
