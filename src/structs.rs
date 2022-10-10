// Copyright © 2020-2022 Jonathan Vasquez <jon@xyinn.org>
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions
// are met:
//
// 1. Redistributions of source code must retain the above copyright
//    notice, this list of conditions and the following disclaimer.
//
// 2. Redistributions in binary form must reproduce the above copyright
//    notice, this list of conditions and the following disclaimer in the
//    documentation and/or other materials provided with the distribution.
//
// THIS SOFTWARE IS PROVIDED BY THE AUTHOR AND CONTRIBUTORS "AS IS" AND
// ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
// IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
// ARE DISCLAIMED.  IN NO EVENT SHALL THE AUTHOR OR CONTRIBUTORS BE LIABLE
// FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
// DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
// OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
// HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
// LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
// OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
// SUCH DAMAGE.

use super::enums::{SystemError, SystemResult};
use super::get_cutoff_date;
use super::traits::Communicator;
use super::SNAPSHOT_FORMAT;
use chrono::prelude::*;
use std::fmt;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::process::{Command, Stdio};
#[derive(Debug)]
pub struct Config {
    pool: String,
    date: DateTime<Local>,
    exclude_file: String,
    show_queued: bool,
    show_excluded: bool,
    dry_run: bool,
    iteration_count: u32,
    no_confirm: bool,
    label: String,
    show_config: bool,
}

impl Config {
    pub fn new<T: Communicator>(
        communicator: &T,
        pool: &str,
        date: &str,
        exclude_file: &str,
        show_queued: bool,
        show_excluded: bool,
        dry_run: bool,
        iteration_count: u32,
        no_confirm: bool,
        label: &str,
        show_config: bool,
    ) -> Config {
        let cutoff_date: DateTime<Local>;
        if date.is_empty() {
            cutoff_date = get_cutoff_date(Local::now());
        } else {
            cutoff_date = match Local.datetime_from_str(&date, SNAPSHOT_FORMAT) {
                Err(_) => panic!("Error parsing date: Example: 2017-09-26-1111-00"),
                Ok(v) => v,
            };
        }
        if !exclude_file.is_empty() {
            if !communicator.does_file_exist(&exclude_file) {
                panic!("File doesn't exist: {}", exclude_file);
            }
        }
        Config {
            pool: pool.to_string(),
            date: cutoff_date,
            exclude_file: exclude_file.to_string(),
            show_queued,
            show_excluded,
            dry_run,
            iteration_count,
            no_confirm,
            label: label.to_string(),
            show_config,
        }
    }

    pub fn print(&self) {
        println!("Configuration");
        println!("----------------");
        println!("Pool: {}", self.pool());
        println!("Cut Off Date: {}", self.date().format(SNAPSHOT_FORMAT));
        println!("Exclude File: {}", self.exclude_file());
        println!("Label (Filter): {}", self.label());
        if self.should_show_config() {
            println!("Show Queued: {}", self.should_show_queued());
            println!("Show Excluded: {}", self.should_show_excluded());
            println!("Dry Run: {}", self.should_dry_run());
            println!("Iteration Amount (Batch): {}", self.iteration_count());
            println!("No Confirmation: {}", self.no_confirm());
            println!("Show Config: {}", self.should_show_config());
        }
        println!("");
    }

    pub fn pool(&self) -> &String {
        &self.pool
    }

    pub fn date(&self) -> &DateTime<Local> {
        &self.date
    }

    pub fn exclude_file(&self) -> &String {
        &self.exclude_file
    }

    pub fn should_show_queued(&self) -> bool {
        self.show_queued
    }

    pub fn should_show_excluded(&self) -> bool {
        self.show_excluded
    }

    pub fn should_dry_run(&self) -> bool {
        self.dry_run
    }

    pub fn iteration_count(&self) -> u32 {
        self.iteration_count
    }

    pub fn no_confirm(&self) -> bool {
        self.no_confirm
    }

    pub fn label(&self) -> &String {
        &self.label
    }

    pub fn should_show_config(&self) -> bool {
        self.show_config
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct Snapshot {
    pool: String,
    dataset: String,
    date: DateTime<Local>,
    label: String,
    suffix: String,
}

impl Snapshot {
    pub fn new(pool: &str, dataset: &str, date: DateTime<Local>, label: &str) -> Snapshot {
        let mut snapshot = Snapshot {
            pool: pool.to_string(),
            dataset: dataset.to_string(),
            date,
            label: label.to_string(),
            suffix: String::new(),
        };

        // Auto-generate the suffix name so we don't have to create
        // multiple string copies later.
        snapshot
            .suffix
            .push_str(snapshot.date.format(SNAPSHOT_FORMAT).to_string().as_str());
        snapshot.suffix.push_str("-");
        snapshot.suffix.push_str(snapshot.label.as_str());
        snapshot
    }

    pub fn is_stale(&self, cutoff_date: &DateTime<Local>) -> bool {
        &self.date < cutoff_date
    }

    pub fn suffix(&self) -> &String {
        &self.suffix
    }

    pub fn pool(&self) -> &String {
        &self.pool
    }

    pub fn dataset(&self) -> &String {
        &self.dataset
    }
    pub fn date(&self) -> &DateTime<Local> {
        &self.date
    }

    pub fn label(&self) -> &String {
        &self.label
    }
}

impl fmt::Display for Snapshot {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}@{}-{}",
            self.dataset,
            self.date.format(SNAPSHOT_FORMAT).to_string(),
            self.label
        )
    }
}

impl fmt::Debug for Snapshot {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}@{}-{}",
            self.dataset,
            self.date.format(SNAPSHOT_FORMAT).to_string(),
            self.label
        )
    }
}

pub struct RealCommunicator;

// Integration Tested Only
impl Communicator for RealCommunicator {
    fn get_snapshots(&self) -> SystemResult {
        // Example: zfs list -t snapshot -H -o name -s name

        let zfs_cmd = match Command::new("zfs")
            .arg("list")
            .arg("-t")
            .arg("snapshot")
            .arg("-H")
            .arg("-o")
            .arg("name")
            .arg("-s")
            .arg("name")
            .stdout(Stdio::piped())
            .spawn()
        {
            Err(e) => return Err(SystemError::SpawnProcess(e.to_string())),
            Ok(p) => p,
        };

        let mut results = String::new();
        match zfs_cmd.stdout.unwrap().read_to_string(&mut results) {
            Err(e) => Err(SystemError::ReadingFromString(e.to_string())),
            Ok(_) => Ok(results),
        }
    }

    fn destroy_snapshots(&self, snapshots: String) -> SystemResult {
        match Command::new("zfs").arg("destroy").arg(&snapshots).status() {
            Ok(_) => Ok(snapshots),
            Err(e) => Err(SystemError::DeleteSnapshots(e.to_string())),
        }
    }

    fn get_excluded_snapshots(&self, exclude_file: &str) -> SystemResult {
        let mut f = match File::open(exclude_file) {
            Err(e) => return Err(SystemError::OpeningFile(e.to_string())),
            Ok(v) => v,
        };

        let mut contents = String::new();

        match f.read_to_string(&mut contents) {
            Err(e) => Err(SystemError::ReadingFromString(e.to_string())),
            Ok(_) => Ok(contents),
        }
    }
    fn does_file_exist(&self, filename: &str) -> bool {
        Path::new(filename).exists()
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::utility::*;
    use super::*;

    mod snapshot {
        use super::*;
        #[test]
        fn is_stale_if_old_should_return_true() {
            let cutoff_date = Local.ymd(2020, 08, 15).and_hms(23, 54, 09);
            let snapshot = create_snapshot("tank/gentoo/os", "2020-07-13-2354-09", "CHECKPOINT");
            assert!(snapshot.is_stale(&cutoff_date));
        }
        #[test]
        fn is_stale_if_new_should_return_false() {
            let cutoff_date = Local.ymd(2020, 08, 15).and_hms(23, 54, 09);
            let snapshot = create_snapshot("tank/gentoo/os", "2020-08-15-2354-09", "CHECKPOINT");
            assert_eq!(snapshot.is_stale(&cutoff_date), false);
        }
    }

    mod config {
        use super::*;
        #[test]
        fn get_config() {
            let communicator = FakeCommunicator::new(true);
            let date = "2099-01-01-0000-00";
            let config = Config::new(
                &communicator,
                "tank",
                date,
                "some-file",
                true,
                true,
                true,
                59,
                true,
                "ANIMALS",
                true,
            );
            assert_eq!(config.pool(), "tank");
            assert_eq!(
                config.date(),
                &Local.datetime_from_str(date, SNAPSHOT_FORMAT).unwrap()
            );
            assert_eq!(config.exclude_file(), "some-file");
            assert_eq!(config.should_show_queued(), true);
            assert_eq!(config.should_show_excluded(), true);
            assert_eq!(config.should_dry_run(), true);
            assert_eq!(config.iteration_count(), 59);
            assert_eq!(config.no_confirm(), true);
            assert_eq!(config.label(), "ANIMALS");
            assert_eq!(config.should_show_config(), true);
        }
        #[test]
        #[should_panic]
        fn config_if_file_doesnt_exist_should_panic() {
            let communicator = FakeCommunicator::new(false);
            Config::new(
                &communicator,
                "tank",
                "2099-01-01-0000-00",
                "some-file",
                true,
                true,
                true,
                59,
                true,
                "ANIMALS",
                true,
            );
        }
    }
}
