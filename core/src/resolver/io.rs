use std::io;

impl ResolverIO for TerminalIO {
    fn write_str(&mut self, s: &str) -> io::Result<()> {
        use std::io::Write;
        let mut stdout = io::stdout().lock();
        stdout.write_all(s.as_bytes())?;
        stdout.flush()
    }

    fn read_line(&mut self) -> io::Result<String> {
        let mut buf = String::new();
        io::stdin().read_line(&mut buf)?;
        Ok(buf)
    }
}

pub trait ResolverIO {
    fn write_str(&mut self, s: &str) -> io::Result<()>;
    fn read_line(&mut self) -> io::Result<String>;
}

pub struct TerminalIO;

#[cfg(test)]
pub use mock::MockIO;

#[cfg(test)]
mod mock {
    use super::*;
    use std::collections::VecDeque;

    impl ResolverIO for MockIO {
        fn write_str(&mut self, s: &str) -> io::Result<()> {
            self.outputs.push(s.to_string());
            Ok(())
        }

        fn read_line(&mut self) -> io::Result<String> {
            self.inputs
                .pop_front()
                .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "no more mock inputs"))
        }
    }

    pub struct MockIO {
        inputs: VecDeque<String>,
        pub outputs: Vec<String>,
    }

    impl MockIO {
        pub fn new(inputs: Vec<&str>) -> Self {
            Self {
                inputs: inputs.into_iter().map(String::from).collect(),
                outputs: Vec::new(),
            }
        }

        pub fn output(&self) -> String {
            self.outputs.join("")
        }
    }
}
