pub mod args {
    use std::ffi::OsString;

    #[cfg(feature = "argfile")]
    pub type Result<A> = std::io::Result<A>;
    #[cfg(not(feature = "argfile"))]
    pub type Result<A> = A;

    pub fn args() -> Result<Vec<OsString>> {
        expand_args(args_os())
    }

    pub fn args_from<T: Into<OsString>, I: Iterator<Item = T>>(args: I) -> Result<Vec<OsString>> {
        expand_args(args.map(|s| s.into()))
    }

    #[cfg(feature = "wild")]
    fn args_os() -> wild::ArgsOs {
        wild::args_os()
    }

    #[cfg(not(feature = "wild"))]
    fn args_os() -> std::env::ArgsOs {
        std::env::args_os()
    }

    #[cfg(feature = "argfile")]
    fn expand_args(args: impl Iterator<Item = OsString>) -> Result<Vec<OsString>> {
        argfile::expand_args_from(args, argfile::parse_fromfile, argfile::PREFIX)
    }

    #[cfg(not(feature = "argfile"))]
    fn expand_args(args: impl Iterator<Item = OsString>) -> Result<Vec<OsString>> {
        args.collect()
    }

    #[cfg(feature = "argfile")]
    pub(crate) fn _with_args<A>(op: impl FnOnce(Vec<OsString>) -> A) -> Result<A> {
        Ok(op(args()?))
    }

    #[cfg(not(feature = "argfile"))]
    pub(crate) fn _with_args<A>(op: impl FnOnce(Vec<OsString>) -> A) -> Result<A> {
        op(args())
    }

    #[cfg(feature = "argfile")]
    pub(crate) fn _with_args_from<A, T, I>(
        args: I,
        op: impl FnOnce(Vec<OsString>) -> A,
    ) -> Result<A>
    where
        T: Into<OsString>,
        I: Iterator<Item = T>,
    {
        Ok(op(args_from(args)?))
    }

    #[cfg(not(feature = "argfile"))]
    pub(crate) fn _with_args_from<A, T, I>(
        args: I,
        op: impl FnOnce(Vec<OsString>) -> A,
    ) -> Result<A>
    where
        T: Into<OsString>,
        I: Iterator<Item = T>,
    {
        op(args_from(args))
    }

    #[cfg(feature = "argfile")]
    pub(crate) fn _into_result<A>(value: A) -> Result<A> {
        Ok(value)
    }

    #[cfg(not(feature = "argfile"))]
    pub(crate) fn _into_result<A>(value: A) -> Result<A> {
        value
    }
}

#[cfg(any(feature = "clap_app_color", feature = "clap_color"))]
pub mod color {
    #[cfg(feature = "clap_color")]
    use concolor::ColorChoice;

    #[cfg(feature = "clap_app_color")]
    pub fn clap_app_color() -> clap::ColorChoice {
        let color = concolor::get(concolor::Stream::Either);
        if color.ansi_color() {
            clap::ColorChoice::Always
        } else {
            clap::ColorChoice::Never
        }
    }

    #[cfg(feature = "clap_color")]
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    #[cfg_attr(feature = "clap_derive", derive(clap::ValueEnum))]
    pub enum Choice {
        Auto,
        Always,
        Never,
    }

    #[cfg(feature = "clap_color")]
    impl Default for Choice {
        fn default() -> Self {
            Self::Auto
        }
    }

    #[cfg(feature = "clap_color")]
    #[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
    #[cfg_attr(feature = "clap_derive", derive(clap::Args))]
    pub struct Color {
        /// Control when to use colors
        #[cfg_attr(
            feature = "clap_derive",
            clap(long, value_name = "WHEN", visible_alias = "colour", default_value_t = Choice::Auto, default_missing_value = "always", value_enum)
        )]
        color: Choice,

        /// Disable the use of color. Implies `--color=never`
        #[cfg_attr(
            feature = "clap_derive",
            clap(long, visible_alias = "no-colour", conflicts_with = "color")
        )]
        no_color: bool,
    }

    #[cfg(feature = "clap_color")]
    impl Color {
        pub fn apply(self, stream: impl Into<Option<concolor::Stream>>) -> concolor::Color {
            let choice = self.as_color_choice();
            concolor::set(choice);
            let stream = stream.into().unwrap_or(concolor::Stream::Either);
            concolor::get(stream)
        }

        pub(crate) fn as_color_choice(self) -> ColorChoice {
            if self.no_color {
                ColorChoice::Never
            } else {
                match self.color {
                    Choice::Auto => ColorChoice::Auto,
                    Choice::Always => ColorChoice::Always,
                    Choice::Never => ColorChoice::Never,
                }
            }
        }
    }
}

#[cfg(feature = "clap_file")]
pub mod filearg {
    use same_file::Handle;
    use std::{
        ffi::OsStr,
        fs::Metadata,
        path::{Path, PathBuf},
        str::FromStr,
    };

    macro_rules! file_arg {
        ($($name:ident($stdx:ident => $hdl:ident)),+ $(,)?) => {
            $(
                #[derive(Debug, Clone, PartialEq, Eq)]
                pub enum $name {
                    $stdx(Option<PathBuf>),
                    File(PathBuf),
                }

                impl $name {
                    pub fn path(&self) -> Option<&Path> {
                        match self {
                            Self::$stdx(path) => path.as_deref(),
                            Self::File(path) => Some(path),
                        }
                    }
                }

                impl FromStr for $name {
                    type Err = std::convert::Infallible;

                    fn from_str(s: &str) -> Result<Self, Self::Err> {
                        Ok(Self::from(OsStr::new(s)))
                    }
                }

                impl From<&OsStr> for $name {
                    fn from(path: &OsStr) -> Self {
                        if path == "-" {
                            Self::$stdx(Handle::$hdl().ok().and_then(path_from_handle))
                        } else {
                            Self::File(Path::new(path).to_path_buf())
                        }
                    }
                }
            )+
        };
    }

    file_arg!(InputFile(Stdin => stdin), OutputFile(Stdout => stdout), ErrorFile(Stderr => stderr));

    impl InputFile {
        pub fn read_to_string(&self) -> super::Result<String> {
            Ok(match self {
                Self::Stdin(Some(path)) | Self::File(path) => ::std::fs::read_to_string(path)?,
                Self::Stdin(None) => std::io::read_to_string(std::io::stdin().lock())?,
            })
        }
    }

    fn path_from_handle(h: Handle) -> Option<PathBuf> {
        use filepath::FilePath;
        let f = h.as_file();
        f.metadata()
            .ok()
            .filter(Metadata::is_file)
            .and_then(|_| f.path().ok())
    }
}

#[cfg(feature = "clap_verbose")]
pub mod verbose {
    use std::marker::PhantomData;

    use crate::Verbosity;

    #[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
    pub struct Local;

    #[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
    pub struct Global;

    pub trait Scope {
        const IS_GLOBAL: bool;
    }

    impl Scope for Local {
        const IS_GLOBAL: bool = false;
    }

    impl Scope for Global {
        const IS_GLOBAL: bool = true;
    }

    #[derive(Copy, Clone, Default, PartialEq, Eq)]
    #[cfg_attr(feature = "clap_derive", derive(clap::Args))]
    pub struct Verbose<S: Scope = Local> {
        /// Print more logs, can be used multiple times
        #[cfg_attr(
            feature = "clap_derive",
            clap(short, long, action = clap::ArgAction::Count, conflicts_with = "quiet", global = S::IS_GLOBAL)
        )]
        verbose: u8,

        /// Print less logs, can be used multiple times
        #[cfg_attr(
            feature = "clap_derive",
            clap(short, long, action = clap::ArgAction::Count, conflicts_with = "verbose", global = S::IS_GLOBAL)
        )]
        quiet: u8,

        #[cfg_attr(feature = "clap_derive", clap(skip))]
        _scope: PhantomData<S>,
    }

    impl<S: Scope> std::fmt::Debug for Verbose<S> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("Verbose")
                .field("verbose", &self.verbose)
                .field("quiet", &self.quiet)
                .finish()
        }
    }

    impl<S: Scope> Verbose<S> {
        pub fn new(verbose: u8, quiet: u8) -> Self {
            Self {
                verbose,
                quiet,
                _scope: PhantomData,
            }
        }

        pub fn verbosity(self) -> Verbosity {
            self.verbosity_with_default_level(Verbosity::Warn)
        }

        pub fn verbosity_with_default_level(self, default: Verbosity) -> Verbosity {
            const VALUES: [Verbosity; 9] = [
                Verbosity::Off,
                Verbosity::Error,
                Verbosity::Warn,
                Verbosity::CrateInfo,
                Verbosity::CrateDebug,
                Verbosity::CrateTrace,
                Verbosity::InfoCrateTrace,
                Verbosity::DebugCrateTrace,
                Verbosity::Trace,
            ];

            let verbosity = self.verbosity_value();

            let default_pos = VALUES.iter().position(|&v| v == default).unwrap();

            let pos = if verbosity >= 0 {
                default_pos
                    .saturating_add(verbosity.unsigned_abs())
                    .min(VALUES.len() - 1)
            } else {
                default_pos.saturating_sub(verbosity.unsigned_abs())
            };

            VALUES[pos]
        }

        pub fn verbosity_value(self) -> isize {
            (isize::from(self.verbose)) - (isize::from(self.quiet))
        }

        pub(crate) fn erase(self) -> Verbose {
            Verbose {
                verbose: self.verbose,
                quiet: self.quiet,
                _scope: PhantomData,
            }
        }
    }
}

#[cfg(feature = "clap")]
pub mod clap_app {
    use clap::{Command, Parser};
    use std::ffi::OsString;

    pub fn get<A: Parser>() -> crate::args::Result<A> {
        crate::args::_with_args(get_from_args)
    }

    pub fn get_from<A, T, I>(args: I) -> crate::args::Result<A>
    where
        A: Parser,
        T: Into<OsString>,
        I: Iterator<Item = T>,
    {
        crate::args::_with_args_from(args, get_from_args)
    }

    pub fn try_get<A: Parser>() -> crate::args::Result<clap::error::Result<A>> {
        crate::args::_with_args(try_get_from_args)
    }

    pub fn try_get_from<A, T, I>(args: I) -> crate::args::Result<clap::error::Result<A>>
    where
        A: Parser,
        T: Into<OsString>,
        I: Iterator<Item = T>,
    {
        crate::args::_with_args_from(args, try_get_from_args)
    }

    pub(crate) fn get_from_args<A: Parser>(args: Vec<OsString>) -> A {
        match try_get_from_args::<A>(args) {
            Ok(res) => res,
            Err(e) => e.exit(),
        }
    }

    pub(crate) fn try_get_from_args<A: Parser>(args: Vec<OsString>) -> Result<A, clap::Error> {
        #[cfg_attr(feature = "clap_app_color", allow(unused_mut))]
        let mut cmd = A::command();
        #[cfg(feature = "clap_app_color")]
        let mut cmd = cmd.color(crate::clap_app_color());

        match try_get_from_cmd_and_args::<A>(&mut cmd, args) {
            Ok(res) => Ok(res),
            Err(e) => {
                let e = e.format(&mut cmd);
                drop(cmd);
                Err(e)
            }
        }
    }

    fn try_get_from_cmd_and_args<A: clap::FromArgMatches>(
        cmd: &mut Command,
        args: Vec<OsString>,
    ) -> Result<A, clap::Error> {
        let matches = cmd.try_get_matches_from_mut(args)?;
        A::from_arg_matches(&matches)
    }
}

#[cfg(any(
    feature = "setup_clap",
    feature = "setup_color-eyre",
    feature = "setup_tracing"
))]
pub mod setup {
    #[cfg(all(feature = "clap_verbose", feature = "setup_tracing"))]
    use crate::verbose::Scope;
    #[cfg(feature = "setup_clap")]
    use clap::Parser;
    #[cfg(feature = "setup_clap")]
    use std::{ffi::OsString, marker::PhantomData};

    #[cfg(all(feature = "clap_verbose", feature = "setup_tracing"))]
    type VerboseArg<A> = (&'static str, Box<dyn FnOnce(&A) -> crate::Verbose>);
    #[cfg(feature = "clap_color")]
    type ColorArg<A> = Box<dyn FnOnce(&A) -> crate::Color>;

    #[cfg(feature = "setup_clap")]
    pub struct SetupClap<A> {
        _app: PhantomData<A>,
        args: Option<Vec<OsString>>,
        #[cfg(all(feature = "clap_verbose", feature = "setup_tracing"))]
        verbose: Option<VerboseArg<A>>,
        #[cfg(feature = "clap_color")]
        color: Option<ColorArg<A>>,
        #[cfg(feature = "clap_color")]
        stream: Option<concolor::Stream>,
    }

    #[cfg(feature = "setup_clap")]
    impl<A: Parser> Default for SetupClap<A> {
        fn default() -> Self {
            Self {
                _app: PhantomData,
                args: None,
                #[cfg(all(feature = "clap_verbose", feature = "setup_tracing"))]
                verbose: None,
                #[cfg(feature = "clap_color")]
                color: None,
                #[cfg(feature = "clap_color")]
                stream: None,
            }
        }
    }

    #[cfg(feature = "setup_clap")]
    impl<A> std::fmt::Debug for SetupClap<A> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let mut d = f.debug_struct("SetupClap");

            d.field("args_defined", &self.args.is_some());

            #[cfg(all(feature = "clap_verbose", feature = "setup_tracing"))]
            d.field("verbose_defined", &self.verbose.is_some());

            #[cfg(feature = "clap_color")]
            d.field("color_defined", &self.color.is_some());

            #[cfg(feature = "clap_color")]
            d.field("stream_defined", &self.stream.is_some());

            d.finish()
        }
    }

    #[cfg(feature = "setup_clap")]
    impl<A: Parser> SetupClap<A> {
        pub fn with_args<T: Into<OsString>, I: Iterator<Item = T>>(
            mut self,
            args: I,
        ) -> crate::args::Result<Self> {
            crate::args::_with_args_from(args, move |args| {
                self.args = Some(args);
                self
            })
        }

        #[cfg(all(feature = "clap_verbose", feature = "setup_tracing"))]
        pub fn verbose_from<S: Scope>(
            mut self,
            pkg_name: &'static str,
            verbose: impl FnOnce(&A) -> crate::Verbose<S> + 'static,
        ) -> Self {
            self.verbose = Some((pkg_name, Box::new(move |a| verbose(a).erase())));
            self
        }

        #[cfg(feature = "clap_color")]
        pub fn color_from(mut self, color: impl FnOnce(&A) -> crate::Color + 'static) -> Self {
            self.color = Some(Box::new(color));
            self
        }

        #[cfg(feature = "clap_color")]
        pub fn color_stream(mut self, stream: concolor::Stream) -> Self {
            self.stream = Some(stream);
            self
        }

        pub fn run(mut self) -> crate::args::Result<A> {
            let args = self.args.take();

            match args {
                Some(args) => crate::args::_into_result(self.run_with_args(args)),
                None => crate::args::_with_args(|args| self.run_with_args(args)),
            }
        }

        fn run_with_args(self, args: Vec<OsString>) -> A {
            let app = crate::clap_app::get_from_args(args);

            #[cfg(feature = "clap_color")]
            {
                if let Some(color) = self.color {
                    let color = color(&app);
                    let stream = self.stream.unwrap_or(concolor::Stream::Either);
                    let _color = color.apply(stream);
                }
            }

            #[cfg(all(feature = "clap_verbose", feature = "setup_tracing"))]
            {
                if let Some((pkg_name, verbose)) = self.verbose {
                    let verbose = verbose(&app);
                    crate::setup_tracing(pkg_name, verbose.verbosity(), BacktraceLevel::default());
                }
            }

            app
        }
    }

    #[cfg(feature = "setup_clap")]
    pub fn clap<A: Parser>() -> SetupClap<A> {
        SetupClap::default()
    }

    #[cfg(feature = "setup_color-eyre")]
    pub fn color_eyre() -> color_eyre::Result<()> {
        color_eyre_builder().install()
    }

    #[cfg(feature = "setup_color-eyre")]
    pub fn color_eyre_builder() -> color_eyre::config::HookBuilder {
        let builder = color_eyre::config::HookBuilder::default().display_env_section(false);

        #[cfg(any(feature = "clap_color", feature = "clap_app_color"))]
        let builder = if concolor::get(concolor::Stream::Stderr).ansi_color() {
            builder
        } else {
            builder.theme(color_eyre::config::Theme::new())
        };

        builder
    }

    #[cfg(feature = "setup_tracing")]
    #[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
    pub enum BacktraceLevel {
        #[default]
        DebugFullReleaseOff,
        DebugSimpleReleaseOff,
        Off,
        DebugFullReleaseSimple,
        Simple,
        Full,
    }

    #[cfg(feature = "setup_tracing")]
    impl BacktraceLevel {
        #[cfg(debug_assertions)]
        fn level(self) -> &'static str {
            match self {
                Self::DebugFullReleaseOff | Self::DebugFullReleaseSimple | Self::Full => "full",
                Self::DebugSimpleReleaseOff | Self::Simple => "1",
                Self::Off => "0",
            }
        }

        #[cfg(not(debug_assertions))]
        fn level(self) -> &'static str {
            match self {
                Self::DebugFullReleaseOff | Self::DebugSimpleReleaseOff | Self::Off => "0",
                Self::DebugFullReleaseSimple | Self::Simple => "1",
                Self::Full => "full",
            }
        }
    }

    #[cfg(feature = "setup_tracing")]
    pub fn tracing(pkg_name: &str, verbosity: crate::Verbosity, level: BacktraceLevel) {
        tracing_filter(&verbosity.as_filter(pkg_name), level)
    }

    #[cfg(feature = "setup_tracing")]
    pub fn tracing_filter(filter: &str, level: BacktraceLevel) {
        use tracing_error::ErrorLayer;
        use tracing_subscriber::prelude::*;
        use tracing_subscriber::{fmt, EnvFilter};

        if std::env::var("RUST_LIB_BACKTRACE").is_err() {
            std::env::set_var("RUST_LIB_BACKTRACE", level.level())
        }
        if std::env::var("RUST_BACKTRACE").is_err() {
            std::env::set_var("RUST_BACKTRACE", level.level())
        }

        let fmt_layer = fmt::layer()
            .with_target(true)
            .with_file(true)
            .with_line_number(true)
            .with_writer(std::io::stderr);

        #[cfg(any(feature = "clap_color", feature = "clap_app_color"))]
        let fmt_layer = fmt_layer.with_ansi(concolor::get(concolor::Stream::Stderr).ansi_color());

        let filter_layer =
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(filter));

        tracing_subscriber::registry()
            .with(filter_layer)
            .with(fmt_layer)
            .with(ErrorLayer::default())
            .init();
    }
}

use std::borrow::Cow;

#[cfg(feature = "clap")]
pub use clap;
#[cfg(feature = "color-eyre")]
pub use color_eyre;
#[cfg(feature = "concolor")]
pub use concolor;
#[cfg(feature = "tracing")]
pub use tracing;

pub use args::args;
#[cfg(feature = "clap")]
pub use clap_app::get as get_clap;
#[cfg(feature = "clap_app_color")]
pub use color::clap_app_color;
#[cfg(feature = "clap_color")]
pub use color::Color;
#[cfg(feature = "clap_file")]
pub use filearg::{ErrorFile, InputFile, OutputFile};
#[cfg(feature = "setup_clap")]
pub use setup::clap as setup_clap;
#[cfg(feature = "setup_color-eyre")]
pub use setup::{color_eyre as setup_color_eyre, color_eyre_builder as setup_color_eyre_builder};
#[cfg(feature = "setup_tracing")]
pub use setup::{tracing as setup_tracing, tracing_filter as setup_tracing_filter, BacktraceLevel};
#[cfg(feature = "clap_verbose")]
pub use verbose::{Global, Local, Verbose};

#[cfg(feature = "color-eyre")]
pub use color_eyre::Result;
#[cfg(not(feature = "color-eyre"))]
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;

#[macro_export]
macro_rules! verbosity_filter {
    ($verbose:expr) => {
        $verbose.as_filter($crate::pkg_name!())
    };
}

#[macro_export]
macro_rules! pkg_name {
    () => {
        ::std::env!("CARGO_PKG_NAME")
    };
}

/// Desired level of verbosity
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Verbosity {
    /// No messages
    Off,
    /// error messages of all targets
    Error,
    /// warn messages of all targets
    Warn,
    /// info messages of all targets
    Info,
    /// debug messages of all targets
    Debug,
    /// trace messages of all targets
    Trace,
    /// info messages for the compiled crate
    CrateInfo,
    /// info messages for the compiled crate
    CrateDebug,
    /// trace messages for the compiled crate
    CrateTrace,
    /// info messages for all tragets, trace messages for the compiled crate
    InfoCrateTrace,
    /// debug messages for all tragets, trace messages for the compiled crate
    DebugCrateTrace,
}

impl Verbosity {
    pub fn as_filter<'a>(self, pkg_name: impl Into<Option<&'a str>>) -> Cow<'static, str> {
        match pkg_name.into() {
            Some(pkg_name) => self.as_filter_for_pkg(pkg_name),
            None => self.as_filter_for_all().into(),
        }
    }

    pub fn as_filter_for_all(self) -> &'static str {
        match self {
            Verbosity::Off => "off",
            Verbosity::Error => "error",
            Verbosity::Warn => "warn",
            Verbosity::CrateInfo => "info",
            Verbosity::CrateDebug => "debug",
            _ => "trace",
        }
    }

    pub fn as_filter_for_pkg(self, pkg_name: &str) -> Cow<'static, str> {
        match self {
            Verbosity::Off => "off".into(),
            Verbosity::Error => "error".into(),
            Verbosity::Warn => "warn".into(),
            Verbosity::Info => "info".into(),
            Verbosity::Debug => "debug".into(),
            Verbosity::Trace => "trace".into(),
            Verbosity::CrateInfo => format!("{}=info", pkg_name).into(),
            Verbosity::CrateDebug => format!("{}=debug", pkg_name).into(),
            Verbosity::CrateTrace => format!("{}=trace", pkg_name).into(),
            Verbosity::InfoCrateTrace => format!("{}=trace,info", pkg_name).into(),
            Verbosity::DebugCrateTrace => format!("{}=trace,debug", pkg_name).into(),
        }
    }
}
