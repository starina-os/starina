//! Cargo's dep-info file parser.
//!
//! <https://doc.rust-lang.org/cargo/reference/build-cache.html#dep-info-files>

pub enum Error {
    DelimiterNotFound,
}

pub struct DepInfoParser<'a> {
    prerequisites: Vec<&'a str>,
}

impl DepInfoParser {
    pub fn parse() -> Result<DepInfoParser, Error> {
        let mut prerequisites = Vec::new();
        let mut parts = input.split(':');
        let target = parts.next().unwrap();
        let prerequisites_str = parts.next().ok_or(Error::DelimiterNotFound)?;
        for prerequisite in prerequisites_str.split_whitespace() {
            prerequisites.push(prerequisite);
        }

        Ok(DepInfoParser { prerequisites })
    }

    pub fn prerequisites(&self) -> &[&str] {
        &self.prerequisites
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn real_world_dep_info() {
        let input = "/home/seiya/starina/target/arm64-user/release/tcpip: /home/seiya/starina/apps/servers/tcpip/main.rs /home/seiya/starina/libs/rust/starina/allocator.rs /home/seiya/starina/libs/rust/starina/arch/arm64/mod.rs /home/seiya/starina/libs/rust/starina/arch/arm64/start.S /home/seiya/starina/libs/rust/starina/arch/arm64/syscall.rs /home/seiya/starina/libs/rust/starina/arch/mod.rs /home/seiya/starina/libs/rust/starina/channel.rs /home/seiya/starina/libs/rust/starina/collections.rs /home/seiya/starina/libs/rust/starina/handle.rs /home/seiya/starina/libs/rust/starina/lib.rs /home/seiya/starina/libs/rust/starina/mainloop.rs /home/seiya/starina/libs/rust/starina/message.rs /home/seiya/starina/libs/rust/starina/panic.rs /home/seiya/starina/libs/rust/starina/poll.rs /home/seiya/starina/libs/rust/starina/print.rs /home/seiya/starina/libs/rust/starina/start.rs /home/seiya/starina/libs/rust/starina/syscall.rs /home/seiya/starina/libs/rust/types/error.rs /home/seiya/starina/libs/rust/types/handle.rs /home/seiya/starina/libs/rust/types/lib.rs /home/seiya/starina/libs/rust/types/message.rs /home/seiya/starina/libs/rust/types/poll.rs /home/seiya/starina/libs/rust/types/syscall.rs";
        let parser = DepInfoParser::parse(input).unwrap();

        assert_eq!(
            parser.prerequisites(),
            &[
                "/home/seiya/starina/apps/servers/tcpip/main.rs",
                "/home/seiya/starina/libs/rust/starina/allocator.rs",
                "/home/seiya/starina/libs/rust/starina/arch/arm64/mod.rs",
                "/home/seiya/starina/libs/rust/starina/arch/arm64/start.S",
                "/home/seiya/starina/libs/rust/starina/arch/arm64/syscall.rs",
                "/home/seiya/starina/libs/rust/starina/arch/mod.rs",
                "/home/seiya/starina/libs/rust/starina/channel.rs",
                "/home/seiya/starina/libs/rust/starina/collections.rs",
                "/home/seiya/starina/libs/rust/starina/handle.rs",
                "/home/seiya/starina/libs/rust/starina/lib.rs",
                "/home/seiya/starina/libs/rust/starina/mainloop.rs",
                "/home/seiya/starina/libs/rust/starina/message.rs",
                "/home/seiya/starina/libs/rust/starina/panic.rs",
                "/home/seiya/starina/libs/rust/starina/poll.rs",
                "/home/seiya/starina/libs/rust/starina/print.rs",
                "/home/seiya/starina/libs/rust/starina/start.rs",
                "/home/seiya/starina/libs/rust/starina/syscall.rs",
                "/home/seiya/starina/libs/rust/types/error.rs",
                "/home/seiya/starina/libs/rust/types/handle.rs",
                "/home/seiya/starina/libs/rust/types/lib.rs",
                "/home/seiya/starina/libs/rust/types/message.rs",
                "/home/seiya/starina/libs/rust/types/poll.rs",
                "/home/seiya/starina/libs/rust/types/syscall.rs",
            ]
        );

    }
}
