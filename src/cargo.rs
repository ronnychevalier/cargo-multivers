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
            .find_map(|message| {
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
            .transpose()
    }
}
