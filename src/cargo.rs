use std::path::PathBuf;

use escargot::CommandMessages;
use escargot::error::CargoError;

pub trait CommandMessagesExt {
    /// Finds the executable artifact in the stream of messages from Cargo while printing rustc messages.
    fn find_executable(self) -> anyhow::Result<Option<PathBuf>>;

    /// Finds executable artifacts in the stream of messages from Cargo while printing rustc messages.
    fn find_executables(self) -> impl Iterator<Item = Result<PathBuf, CargoError>>;
}

impl CommandMessagesExt for CommandMessages {
    fn find_executables(self) -> impl Iterator<Item = Result<PathBuf, CargoError>> {
        self.into_iter().filter_map(|message| {
            let message = match message {
                Ok(message) => message,
                Err(e) => return Some(Err(e)),
            };
            match message.decode() {
                Ok(escargot::format::Message::CompilerArtifact(artifact)) => artifact
                    .executable
                    .as_deref()
                    .map(ToOwned::to_owned)
                    .map(Ok),
                Ok(escargot::format::Message::CompilerMessage(e)) => {
                    // We ignore the messages that are generated due to the use of `-Ctarget-feature`
                    if e.message
                        .message
                        .contains("unstable feature specified for `-Ctarget-feature`")
                    {
                        return None;
                    }
                    // We also the "N warnings emitted" messages, because they are no longer accurate
                    // since we ignore the "unstable feature specified for `-Ctarget-feature`" messages.
                    if e.message.message.contains("warning emitted")
                        || e.message.message.contains("warnings emitted")
                    {
                        return None;
                    }
                    if let Some(rendered) = e.message.rendered {
                        eprint!("{rendered}");
                    }

                    None
                }
                Ok(_) => {
                    // Ignored
                    None
                }
                Err(e) => Some(Err(e)),
            }
        })
    }

    fn find_executable(self) -> anyhow::Result<Option<PathBuf>> {
        let found = self.find_executables().collect::<Result<Vec<_>, _>>()?;
        match &found[..] {
            [] => Ok(None),
            [path] => Ok(Some(path.clone())),
            _ => anyhow::bail!(
                "More than one executable built, missing binary selection. Select one using something like `cargo multivers -- --bin my_bin`"
            ),
        }
    }
}
