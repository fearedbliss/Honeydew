// Copyright (C) 2020 Jonathan Vasquez <jon@xyinn.org>

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

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
    iteration_count: u16,
    confirm: bool,
    label: String,
    show_config: bool,
}

impl Config {
    // Integration Tested Only
    pub fn new(
        pool: String,
        date: String,
        exclude_file: Option<String>,
        show_queued: bool,
        show_excluded: bool,
        dry_run: bool,
        iteration_count: u16,
        confirm: bool,
        label: String,
        show_config: bool,
    ) -> Config {
        if pool.is_empty() {
            panic!("Pool name not provided. Example: -p tank");
        }
        let cutoff_date: DateTime<Local>;
        if date.is_empty() {
            cutoff_date = get_cutoff_date(Local::now());
        } else {
            cutoff_date = match Local.datetime_from_str(&date, SNAPSHOT_FORMAT) {
                Err(_) => panic!("Error parsing date: Example: 2017-09-26-1111-00"),
                Ok(v) => v,
            };
        }

        let exclude_file = match exclude_file {
            Some(v) => v,
            None => "".to_string(),
        };
        if !exclude_file.is_empty() {
            if !Path::new(&exclude_file).exists() {
                panic!("File doesn't exist: {}", exclude_file);
            }
        }
        Config {
            pool,
            date: cutoff_date,
            exclude_file,
            show_queued,
            show_excluded,
            dry_run,
            iteration_count,
            confirm,
            label,
            show_config,
        }
    }

    pub fn print(&self) {
        println!("Configuration");
        println!("----------------");
        if self.should_show_config() {
            println!("Pool: {}", self.pool());
            println!("Cut Off Date: {}", self.date().format(SNAPSHOT_FORMAT));
            println!("Exclude File: {}", self.exclude_file());
            println!("Show Queued: {}", self.should_show_queued());
            println!("Show Excluded: {}", self.should_show_excluded());
            println!("Dry Run: {}", self.should_dry_run());
            println!("Iteration Amount (Batch): {}", self.iteration_count());
            println!("Ask For Confirmation: {}", self.confirm());
            println!("Label (Filter): {}", self.label());
            println!("Show Config: {}", self.should_show_config());
        } else {
            println!("Pool: {}", self.pool());
            println!("Cut Off Date: {}", self.date().format(SNAPSHOT_FORMAT));
            println!("Exclude File: {}", self.exclude_file());
            println!("Label (Filter): {}", self.label());
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

    pub fn iteration_count(&self) -> u16 {
        self.iteration_count
    }

    pub fn confirm(&self) -> bool {
        self.confirm
    }

    pub fn label(&self) -> &String {
        &self.label
    }

    pub fn should_show_config(&self) -> bool {
        self.show_config
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
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
}

#[cfg(test)]
mod test {
    use super::super::testing::utility::create_snapshot;
    use super::*;

    mod snapshot {
        use super::*;
        #[test]
        fn is_stale_if_old_should_return_false() {
            let cutoff_date = Local.ymd(2020, 08, 15).and_hms(23, 54, 09);
            let snapshot = create_snapshot("tank/gentoo/os", "2020-07-13-2354-09", "CHECKPOINT");
            assert!(snapshot.is_stale(&cutoff_date));
        }
        #[test]
        fn is_stale_if_new_should_return_true() {
            let cutoff_date = Local.ymd(2020, 08, 15).and_hms(23, 54, 09);
            let snapshot = create_snapshot("tank/gentoo/os", "2020-08-15-2354-09", "CHECKPOINT");
            assert_eq!(snapshot.is_stale(&cutoff_date), false);
        }
    }
}