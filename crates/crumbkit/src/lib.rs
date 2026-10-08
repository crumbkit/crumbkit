//! crumbkit will provide conventions for command-line tools built on clap.
//! A tool keeps its own clap types and its own `main`, and adopts only the
//! parts it wants: compact output, a help tree that reveals one level at a
//! time, and failures with stable codes and a next step.
