use std::path::PathBuf;

use escargot::error::CargoError;
use escargot::CommandMessages;

pub trait CommandMessagesExt {
    /// Finds the executable artifact in the stream of messages from Cargo while printing rustc messages.
    fn find_executable(self) -> Result<Option<PathBuf>, CargoError>;
}

impl CommandMessagesExt for CommandMessages {
    fn find_executable(self) -> Result<Option<PathBuf>, CargoError> {
        self.into_iter()
            .filter_map(|message| {
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
            .last()
            .transpose()
    }
}
