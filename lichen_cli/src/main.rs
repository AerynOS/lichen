// SPDX-FileCopyrightText: Copyright © 2025 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

//! Super basic CLI runner for lichen

use std::{
    io::Write,
    path::PathBuf,
    process::{Command, Output, Stdio},
    str::FromStr,
    time::Duration,
};

use color_eyre::eyre::bail;
use console::{set_colors_enabled, style};
use crossterm::style::Stylize;
use indicatif::ProgressStyle;
use installer::{
    selections::{self, Group},
    steps::Context,
    systemd, Account, BootPartition, Installer, Locale, SystemPartition,
};
use nix::libc::geteuid;

#[derive(Debug)]
struct CliContext {
    root: PathBuf,
}

impl<'a> Context<'a> for CliContext {
    /// Return root of our ops
    fn root(&'a self) -> &'a PathBuf {
        &self.root
    }

    /// Run a step command
    /// Right now all output is dumped to stdout/stderr
    fn run_command(&self, cmd: &mut Command) -> Result<(), installer::steps::Error> {
        let status = cmd.spawn()?.wait()?;
        if !status.success() {
            let program = cmd.get_program().to_string_lossy().into();
            return Err(installer::steps::Error::CommandFailed { program, status });
        }
        Ok(())
    }

    /// Run a astep command, capture stdout
    fn run_command_captured(&self, cmd: &mut Command, input: Option<&str>) -> Result<Output, installer::steps::Error> {
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        let mut ps = cmd.spawn()?;
        let mut stdin = ps.stdin.take().expect("stdin failure");

        if let Some(input) = input {
            stdin.write_all(input.as_bytes())?;
        }
        drop(stdin);

        let output = ps.wait_with_output()?;
        Ok(output)
    }
}

/// Ask the user what locale to use
fn ask_locale<'a>(locales: &'a [Locale<'a>]) -> color_eyre::Result<&'a Locale<'a>> {
    let locales_disp = locales.iter().enumerate().map(|(i, l)| (i, l, "")).collect::<Vec<_>>();
    let index = cliclack::select("Pick a locale")
        .items(locales_disp.as_slice())
        .initial_value(0)
        .filter_mode()
        .set_size(20)
        .interact()?;

    Ok(&locales[index])
}

fn ask_timezone() -> color_eyre::Result<String> {
    let variants = chrono_tz::TZ_VARIANTS
        .iter()
        .enumerate()
        .map(|(i, v)| (i, v, ""))
        .collect::<Vec<_>>();
    let index = cliclack::select("Pick a timezone")
        .items(variants.as_slice())
        .initial_value(0)
        .filter_mode()
        .set_size(10)
        .interact()?;

    Ok(chrono_tz::TZ_VARIANTS[index].to_string())
}

/// Pick an ESP please...
fn ask_esp(parts: &[BootPartition]) -> color_eyre::Result<&BootPartition> {
    let parts_disp = parts
        .iter()
        .enumerate()
        .map(|(i, p)| (i, p.to_string(), ""))
        .collect::<Vec<_>>();
    let index = cliclack::select("Pick a boot partition")
        .items(parts_disp.as_slice())
        .initial_value(0)
        .interact()?;
    Ok(&parts[index])
}

/// Where's it going?
fn ask_rootfs(parts: &[SystemPartition]) -> color_eyre::Result<&SystemPartition> {
    let parts_disp = parts
        .iter()
        .enumerate()
        .map(|(i, p)| (i, p.to_string(), ""))
        .collect::<Vec<_>>();
    let index = cliclack::select("Pick a suitably sized partition for the root filesystem")
        .items(parts_disp.as_slice())
        .initial_value(0)
        .interact()?;
    Ok(&parts[index])
}

fn ask_filesystem() -> color_eyre::Result<String> {
    let variants = [
        ("xfs", "XFS", "For reliability (journaling)"),
        ("f2fs", "F2FS", "For flash storage (speed)"),
        ("ext4", "Ext4", "Not recommended due to small hardlink limit"),
    ];
    let index = cliclack::select("Pick a filesystem for your rootfs")
        .items(&variants)
        .initial_value("xfs")
        .interact()?;
    Ok(index.into())
}

// Grab a password for the root account
fn ask_password() -> color_eyre::Result<String> {
    let password = cliclack::password("You'll need to set a default root (administrator) password").interact()?;
    let confirmed = cliclack::password("Confirm your password")
        .validate_interactively(move |v: &String| {
            if *v != password {
                return Err("Those passwords do not match");
            }
            Ok(())
        })
        .interact()?;
    Ok(confirmed)
}

fn create_user() -> color_eyre::Result<Account> {
    cliclack::log::info("We now need to create a default (admin) user")?;
    let username: String = cliclack::input("Username?").interact()?;
    let password = cliclack::password("Pick a password").interact()?;
    let confirmed = cliclack::password("Now confirm the password")
        .validate_interactively(move |v: &String| {
            if *v != password {
                return Err("Those passwords do not match");
            }
            Ok(())
        })
        .interact()?;
    Ok(Account::new(username)
        .with_password(confirmed)
        .with_shell("/usr/bin/bash"))
}

fn ask_desktop<'a>(desktops: &'a [&Group]) -> color_eyre::Result<&'a selections::Group> {
    let displayable = desktops
        .iter()
        .enumerate()
        .map(|(i, d)| (i, &d.summary, &d.description))
        .collect::<Vec<_>>();
    let index = cliclack::select("Pick a desktop environment to use")
        .items(displayable.as_slice())
        .initial_value(1)
        .interact()?;

    Ok(desktops[index])
}

fn main() -> color_eyre::Result<()> {
    env_logger::init();
    color_eyre::install().unwrap();
    set_colors_enabled(true);

    let euid = unsafe { geteuid() };
    if euid != 0 {
        bail!("lichen must be run as root. Re-run with sudo")
    }

    // Test selection management, force GNOME
    let selections = selections::Manager::new().with_groups([
        selections::Group::from_str(include_str!("../../selections/base.json"))?,
        selections::Group::from_str(include_str!("../../selections/cosmic.json"))?,
        selections::Group::from_str(include_str!("../../selections/develop.json"))?,
        selections::Group::from_str(include_str!("../../selections/gnome.json"))?,
        selections::Group::from_str(include_str!("../../selections/kernel-common.json"))?,
        selections::Group::from_str(include_str!("../../selections/kernel-desktop.json"))?,
    ]);

    let desktops = selections
        .groups()
        .filter(|g| g.name == "cosmic" || g.name == "gnome")
        .collect::<Vec<_>>();

    let sp = cliclack::spinner();
    sp.start("Loading");

    // Load all the things
    let inst = Installer::new()?;
    let boots = inst.boot_partitions();
    let parts = inst.system_partitions();
    let locales = inst.locales_for_ids(systemd::localectl_list_locales()?)?;

    sp.clear();

    cliclack::intro(style("Install AerynOS").bold())?;
    cliclack::log::warning(format!(
        "{} - this is an alpha quality installer.",
        style("Be warned!").bold()
    ))?;

    let selected_desktop = ask_desktop(&desktops)?;
    let selected_locale = ask_locale(&locales)?;
    let timezone = ask_timezone()?;
    let rootpw = ask_password()?;
    let user_account = create_user()?;

    cliclack::log::info("We)'ll now configure your disk")?;
    let esp = ask_esp(boots)?;
    let mut rootfs = ask_rootfs(parts)?.clone();
    rootfs.mountpoint = Some("/".into());
    let fs = ask_filesystem()?;

    let summary = |title: &str, value: &str| format!("{}: {}", style(title).bold(), value);

    let note = [
        summary("Locale", &selected_locale.to_string()),
        summary("Timezone", &timezone),
        summary("Bootloader", &esp.to_string()),
        summary("Root (/) partition", &rootfs.to_string()),
        summary("Root (/) filesystem", &fs),
    ];

    cliclack::note("Installation summary", note.join("\n"))?;

    let model = installer::Model {
        accounts: [Account::root().with_password(rootpw), user_account].into(),
        boot_partition: esp.to_owned(),
        partitions: [rootfs.clone()].into(),
        locale: Some(selected_locale),
        timezone: Some(timezone),
        rootfs_type: fs,
        packages: selections.selections_with(["develop", &selected_desktop.name, "kernel-desktop"])?,
    };

    let y = cliclack::confirm("Do you want to install?").interact()?;
    if !y {
        cliclack::outro_cancel("No changes have been made to your system")?;
        return Ok(());
    }

    cliclack::outro("Now proceeding with installation")?;

    // TODO: Use proper temp directory
    let context = CliContext {
        root: "/tmp/lichen".into(),
    };
    let (cleanups, steps) = inst.compile_to_steps(&model, &context)?;
    let multi = indicatif::MultiProgress::new();
    let total = indicatif::ProgressBar::new(steps.len() as u64 + cleanups.len() as u64).with_style(
        ProgressStyle::with_template("\n|{bar:20.cyan/blue}| {pos}/{len}")
            .unwrap()
            .progress_chars("■≡=- "),
    );

    let total = multi.add(total);
    for step in steps {
        total.inc(1);
        if step.is_indeterminate() {
            let progress_bar = multi.insert_before(
                &total,
                indicatif::ProgressBar::new(1)
                    .with_message(format!("{} {}", step.title().blue(), step.describe().bold(),))
                    .with_style(
                        ProgressStyle::with_template(" {spinner} {wide_msg} ")
                            .unwrap()
                            .tick_chars("--=≡■≡=--"),
                    ),
            );
            progress_bar.enable_steady_tick(Duration::from_millis(150));
            step.execute(&context)?;
        } else {
            multi.println(format!("{} {}", step.title().blue(), step.describe().bold()))?;
            multi.suspend(|| step.execute(&context))?;
        }
    }

    // Execute all the cleanups
    for cleanup in cleanups {
        let progress_bar = multi.insert_before(
            &total,
            indicatif::ProgressBar::new(1)
                .with_message(format!("{} {}", cleanup.title().yellow(), cleanup.describe().bold(),))
                .with_style(
                    ProgressStyle::with_template(" {spinner} {wide_msg} ")
                        .unwrap()
                        .tick_chars("--=≡■≡=--"),
                ),
        );
        progress_bar.enable_steady_tick(Duration::from_millis(150));
        total.inc(1);
        cleanup.execute(&context)?;
    }

    multi.clear()?;
    println!(
        "🎉 🥳 Succesfully installed {}! Reboot now to start using it!",
        style("AerynOS").bold()
    );

    Ok(())
}
