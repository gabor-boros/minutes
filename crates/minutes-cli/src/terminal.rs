//! Terminal cleanup helpers.
//!
//! Interactive prompts (`dialoguer`) hide the cursor while waiting for input.
//! A Ctrl+C during that window raises SIGINT before the prompt can call
//! `show_cursor`, which leaves the user's shell without a visible cursor.

use console::Term;

/// Restores a visible cursor on stdout and stderr.
pub fn restore_cursor() {
    let _ = Term::stderr().show_cursor();
    let _ = Term::stdout().show_cursor();
}

/// Installs a Ctrl+C handler that restores the cursor, then exits with 130.
///
/// # Errors
///
/// Returns an error when another handler is already registered.
pub fn install_interrupt_handler() -> Result<(), ctrlc::Error> {
    ctrlc::set_handler(|| {
        restore_cursor();
        // Move to a fresh line so the shell prompt is not glued to the prompt.
        let _ = Term::stderr().write_line("");
        std::process::exit(130);
    })
}
