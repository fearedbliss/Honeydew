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

pub mod enums;
pub mod structs;
pub mod testing;
pub mod traits;

use argparse::{ArgumentParser, Store, StoreFalse, StoreTrue};
use chrono::prelude::*;
use chrono::Duration;
use enums::SystemResult;
use std::collections::HashSet;
use std::io;
use std::io::prelude::*;
use structs::{Config, RealCommunicator, Snapshot};
use traits::Communicator;

const SNAPSHOT_FORMAT: &str = "%Y-%m-%d-%H%M-%S";

fn print_header() {
    println!("------------------------------");
    println!("Honeydew - v0.7.2");
    println!("Jonathan Vasquez <jon@xyinn.org>");
    println!("Apache License 2.0");
    println!("------------------------------\n");
}

// Integration Tested Only
pub fn run() {
    print_header();
    let communicator = RealCommunicator;
    let config = parse_arguments();

    config.print();

    let excluded_snapshots: Vec<Snapshot>;
    if config.exclude_file().is_empty() {
        excluded_snapshots = Vec::new();
    } else {
        excluded_snapshots = get_excluded_snapshots(&communicator, &config);
    }

    let stale_snapshots = get_relevant_snapshots(&communicator, &config, &excluded_snapshots);

    if config.should_show_queued() {
        println!("These snapshots are QUEUED for REMOVAL:");
        println!("----------------");
        for snapshot_to_delete in &stale_snapshots {
            println!("{}", snapshot_to_delete);
        }
        println!("");
    }

    if config.should_show_excluded() {
        println!("These snapshots are EXCLUDED from REMOVAL:");
        println!("----------------");
        for snapshot_to_exclude in &excluded_snapshots {
            println!("{}", snapshot_to_exclude);
        }
        println!("");
    }

    println!("Amount of Snapshots to Remove: {}", stale_snapshots.len());
    println!(
        "Amount of Snapshots to Exclude: {}",
        excluded_snapshots.len()
    );
    println!("");

    if !config.should_dry_run() {
        if stale_snapshots.len() == 0 {
            println!("Your pool is already clean. Take care!");
            return;
        }

        if !config.confirm() {
            destroy_snapshots(&communicator, &stale_snapshots, config.iteration_count());
            return;
        }
        print!("Do you want to delete the above snapshots? [y/N]: ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(_) => (),
            Err(e) => panic!("Invalid Input. Exiting. Error: {}", e),
        };
        println!("");
        if input.trim().eq_ignore_ascii_case("y") {
            destroy_snapshots(&communicator, &stale_snapshots, config.iteration_count());
        } else {
            println!("Nothing will be deleted. Take care!");
        }
    }
}

// Integration Tested Only
pub fn parse_arguments() -> Config {
    let mut date_string = String::new();
    let mut pool_name = String::new();
    let mut exclude_file = String::new();
    let mut show_queued = false;
    let mut show_excluded = false;
    let mut dry_run = false;
    let mut iteration_count = 100;
    let mut confirm = true;
    let mut label = String::new();
    let mut show_config = false;

    {
        let mut parser = ArgumentParser::new();
        parser.set_description("Honeydew: A simple snapshot cleaner for ZFS.");
        parser
            .refer(&mut pool_name)
            .add_option(&["-p", "--pool"], Store, "The pool you want to clean.")
            .required();
        parser.refer(&mut date_string).add_option(
            &["-d", "--date"],
            Store,
            "The slice date that you want to use as your end point for snapshot deletions.",
        );
        parser.refer(&mut exclude_file).add_option(
            &["-e", "--exclude-file"],
            Store,
            "Excludes the list of snapshots in this file (one snapshot per line).",
        );
        parser.refer(&mut show_queued).add_option(
            &["-s", "--show-queued"],
            StoreTrue,
            "Show snapshots that will be removed.",
        );
        parser.refer(&mut show_excluded).add_option(
            &["-x", "--show-excluded"],
            StoreTrue,
            "Show snapshots that will be excluded.",
        );
        parser.refer(&mut dry_run).add_option(
            &["-n", "--dry-run"],
            StoreTrue,
            "Performs a dry run. No deletions will occur.",
        );
        parser.refer(&mut iteration_count).add_option(
            &["-i", "--per-iteration"],
            Store,
            "Number of snapshots to delete per iteration.",
        );
        parser.refer(&mut confirm).add_option(
            &["-f", "--no-confirm"],
            StoreFalse,
            "Should confirmation be prompted before deleting the snapshots? Used primarily for cron.",
        );
        parser.refer(&mut label).add_option(
            &["-l", "--label"],
            Store,
            "The label of the snapshots that should be cleaned.",
        );
        parser.refer(&mut show_config).add_option(
            &["-c", "--show-config"],
            StoreTrue,
            "Displays the full configuration options used by the application.",
        );
        parser.parse_args_or_exit();
    }

    Config::new(
        pool_name,
        date_string,
        Some(exclude_file),
        show_queued,
        show_excluded,
        dry_run,
        iteration_count,
        confirm,
        label,
        show_config,
    )
}

/// Returns all the snapshots that will be deleted
fn get_relevant_snapshots<T>(
    communicator: &T,
    config: &Config,
    excluded_snapshots: &Vec<Snapshot>,
) -> Vec<Snapshot>
where
    T: Communicator,
{
    let unparsed_snapshots = get_snapshots(communicator);
    let parsed_snapshots = get_parsed_snapshots(unparsed_snapshots);
    let snapshots = get_snapshots_for(&config.pool(), config.label(), parsed_snapshots);
    let stale_snapshots = get_stale_snapshots(snapshots, &config.date());
    remove_excluded_snapshots(stale_snapshots, &excluded_snapshots)
}

fn remove_excluded_snapshots(
    mut snapshots: Vec<Snapshot>,
    excluded_snapshots: &Vec<Snapshot>,
) -> Vec<Snapshot> {
    for excluded_snapshot in excluded_snapshots {
        snapshots.retain(|snapshot| snapshot != excluded_snapshot);
    }
    snapshots
}

/// Retrieves all of the excluded snapshots that are relevant to this pool.
fn get_excluded_snapshots<T>(communicator: &T, config: &Config) -> Vec<Snapshot>
where
    T: Communicator,
{
    let results = communicator.get_excluded_snapshots(config.exclude_file());
    get_snapshots_for(
        config.pool(),
        config.label(),
        get_parsed_snapshots(get_snapshots_base(results)),
    )
}

fn get_snapshots_for(pool: &str, label: &str, snapshots: Vec<Snapshot>) -> Vec<Snapshot> {
    if label.is_empty() {
        snapshots
            .into_iter()
            .filter(|snapshot| snapshot.pool() == pool)
            .collect()
    } else {
        snapshots
            .into_iter()
            .filter(|snapshot| snapshot.pool() == pool && snapshot.label() == label)
            .collect()
    }
}

/// Parses a string into proper Snapshot struct.
/// Returns None if it failed to be parsed.
/// Format: boot@2020-08-12-1237-49-CHECKPOINT
fn parse_snapshot(snapshot: &str) -> Option<Snapshot> {
    // Split the main two sections [name / time-label]
    let initial_split: Vec<_> = snapshot.split("@").collect();

    if initial_split.len() != 2 {
        return None;
    }

    // Extract the pool and dataset name
    let name_splinters: Vec<_> = initial_split[0].split("/").collect();
    let pool = name_splinters[0];
    let dataset = initial_split[0];

    // Extract the time and label
    let date_label_splinters: Vec<_> = initial_split[1].split("-").collect();

    if date_label_splinters.len() != 6 {
        eprintln!("Snapshot is invalid. Skipping: {}", snapshot);
        return None;
    }
    let label = date_label_splinters[date_label_splinters.len() - 1];

    let mut date_string = String::new();

    // year + month + day + time + second
    date_string.push_str(date_label_splinters[0]);
    date_string.push_str("-");
    date_string.push_str(date_label_splinters[1]);
    date_string.push_str("-");
    date_string.push_str(date_label_splinters[2]);
    date_string.push_str("-");
    date_string.push_str(date_label_splinters[3]);
    date_string.push_str("-");
    date_string.push_str(date_label_splinters[4]);

    let date = match Local.datetime_from_str(&date_string, SNAPSHOT_FORMAT) {
        Ok(d) => d,
        Err(e) => {
            eprintln!(
                "Snapshot is invalid. Failed to parse DateTime: {}. Skipping: {}. Error: {}",
                &date_string, snapshot, e
            );

            return None;
        }
    };

    Some(Snapshot::new(pool, dataset, date, label))
}

fn get_parsed_snapshots(unparsed_snapshots: Vec<String>) -> Vec<Snapshot> {
    let mut parsed_snapshots: Vec<Snapshot> = Vec::new();

    for us in unparsed_snapshots {
        let ps = match parse_snapshot(&us) {
            None => continue,
            Some(s) => s,
        };

        parsed_snapshots.push(ps);
    }
    parsed_snapshots
}

fn get_stale_snapshots(snapshots: Vec<Snapshot>, cutoff_date: &DateTime<Local>) -> Vec<Snapshot> {
    snapshots
        .into_iter()
        .filter(|snapshot| snapshot.is_stale(cutoff_date))
        .collect()
}

fn get_snapshots<T>(communicator: &T) -> Vec<String>
where
    T: Communicator,
{
    let results = communicator.get_snapshots();

    get_snapshots_base(results)
}

fn get_snapshots_base(results: SystemResult) -> Vec<String> {
    let results = match results {
        Err(e) => panic!("{:?}", e),
        Ok(v) => v,
    };

    let mut snapshots: Vec<String> = Vec::new();

    for r in results.lines() {
        snapshots.push(r.to_string());
    }

    snapshots
}

fn build_list_to_delete(snapshots: &Vec<&Snapshot>) -> String {
    let mut names = String::new();
    for (index, snapshot) in snapshots.iter().enumerate() {
        if names.is_empty() {
            names.push_str(&snapshot.to_string());
        } else {
            names.push_str(snapshot.suffix().as_str());
        }

        if index + 1 != snapshots.len() {
            names.push_str(",");
        }
    }
    names
}

fn build_destroy_and_clear<'a, T>(
    communicator: &T,
    snapshots: &Vec<&'a Snapshot>,
    numerator: f64,
    denominator: f64,
) -> Vec<&'a Snapshot>
where
    T: Communicator,
{
    let deleted_snapshots = build_and_destroy(communicator, snapshots);
    let percent_completed = calculate_percentage(numerator, denominator);
    println!(
        "Deleted | {:6.2}% <=> [{}/{}]",
        percent_completed, numerator, denominator,
    );
    deleted_snapshots
}

fn build_and_destroy<'a, T>(communicator: &T, snapshots: &Vec<&'a Snapshot>) -> Vec<&'a Snapshot>
where
    T: Communicator,
{
    match communicator.destroy_snapshots(build_list_to_delete(&snapshots)) {
        Err(e) => panic!("{:?}", e),
        Ok(_) => {
            let mut deleted_snapshots: Vec<&Snapshot> = Vec::new();
            for snapshot in snapshots {
                deleted_snapshots.push(snapshot);
            }
            deleted_snapshots
        }
    }
}

fn get_datasets(snapshots: &Vec<Snapshot>) -> HashSet<String> {
    let mut datasets = HashSet::new();

    for snapshot in snapshots {
        if !datasets.contains(snapshot.dataset()) {
            datasets.insert(snapshot.dataset().clone());
        }
    }

    datasets
}

fn get_cutoff_date(time: DateTime<Local>) -> DateTime<Local> {
    time - Duration::days(30)
}

/// Calculates the percentage complete
fn calculate_percentage(numerator: f64, denominator: f64) -> f64 {
    numerator / denominator * 100.0
}
// NOTE: Having too high of a snapshot deleting amount seems to cause zfs to lock up in some cases (ZFS/Linux Issue).
// Cleaning: zfs destroy <dataset>@<label1>,<label2>,<label3> (Allows us to batch pass the snapshots. Faster.)
fn destroy_snapshots<'a, T>(
    communicator: &T,
    snapshots: &'a Vec<Snapshot>,
    iteration_amount: u16,
) -> Vec<&'a Snapshot>
where
    T: Communicator,
{
    let mut total_processed = 0;
    let snapshot_count = snapshots.len();
    let mut queued_snapshots: Vec<&Snapshot> = Vec::new();
    let mut deleted_snapshots: Vec<&Snapshot> = Vec::new();

    // Snapshots deleted per round need to be all in the same dataset
    // since it will be batched to ZFS for optimization.
    for dataset in get_datasets(&snapshots) {
        println!("Cleaning snapshots for {} ...\n", dataset);
        let snapshots_for_dataset: Vec<&Snapshot> = snapshots
            .iter()
            .filter(|snapshot| snapshot.dataset() == &dataset)
            .collect();

        for (index, snapshot) in snapshots_for_dataset.iter().enumerate() {
            queued_snapshots.push(snapshot);
            if (total_processed as u16 % iteration_amount == 0)
                && (index != 0 || iteration_amount == 1)
            {
                total_processed += queued_snapshots.len();
                build_destroy_and_clear(
                    communicator,
                    &queued_snapshots,
                    total_processed as f64,
                    snapshot_count as f64,
                );
                deleted_snapshots.append(&mut queued_snapshots);
            }

            // If we are about to be done with this dataset, then clean the chamber if needed. ;..;
            if index + 1 == snapshots_for_dataset.len() && queued_snapshots.len() != 0 {
                total_processed += queued_snapshots.len();
                build_destroy_and_clear(
                    communicator,
                    &queued_snapshots,
                    total_processed as f64,
                    snapshot_count as f64,
                );
                deleted_snapshots.append(&mut queued_snapshots);
            }
        }

        println!("");
    }

    if queued_snapshots.len() != 0 {
        // We should never get here if the program is behaving correctly.
        // All the snapshots should be completely deleted by this point.
        println!("These were the remaining snapshots:");
        println!("----------------");
        for snapshot in &queued_snapshots {
            println!("{}", snapshot);
        }
        panic!(
            "There are still {} snapshots in the queue! Please file a bug report!\n",
            queued_snapshots.len()
        )
    }
    deleted_snapshots
}

#[cfg(test)]
mod test {
    use super::*;
    use testing::utility;
    use testing::utility::{create_snapshot, FakeCommunicator};

    #[test]
    fn get_parsed_snapshots_test() {
        let unparsed_snapshots = vec![
            "boot@2020-08-12-1237-49-CHECKPOINT".to_string(),
            "backup/tank/gentoo/home@2020-07-13-2354-09-CHECKPOINT".to_string(),
            "tank/gentoo/os@2020-08-13-2354-09-CHECKPOINT".to_string(),
            "tank@lol".to_string(),
        ];

        let expected_snapshots = vec![
            create_snapshot("boot", "2020-08-12-1237-49", "CHECKPOINT"),
            create_snapshot(
                "backup/tank/gentoo/home",
                "2020-07-13-2354-09",
                "CHECKPOINT",
            ),
            create_snapshot("tank/gentoo/os", "2020-08-13-2354-09", "CHECKPOINT"),
        ];

        let result = get_parsed_snapshots(unparsed_snapshots);
        assert_eq!(expected_snapshots, result);
    }
    #[test]
    fn get_stale_snapshots_test() {
        let snapshots = vec![
            create_snapshot("tank/gentoo/os", "2020-07-13-2354-09", "CHECKPOINT"),
            create_snapshot("tank/gentoo/os", "2020-08-13-2354-09", "CHECKPOINT"),
            create_snapshot("tank/gentoo/os", "2020-09-13-2354-09", "CHECKPOINT"),
        ];

        let cutoff_date = Local.ymd(2020, 09, 10).and_hms(0, 0, 0);

        let expected_snapshots = vec![
            create_snapshot("tank/gentoo/os", "2020-07-13-2354-09", "CHECKPOINT"),
            create_snapshot("tank/gentoo/os", "2020-08-13-2354-09", "CHECKPOINT"),
        ];
        let stale_snapshots = get_stale_snapshots(snapshots, &cutoff_date);

        assert_eq!(expected_snapshots, stale_snapshots);
    }
    #[test]
    fn parse_snapshot_should_return_none() {
        let snapshot = "boot@lol";

        let result = parse_snapshot(&snapshot);

        assert_eq!(None, result);
    }
    #[test]
    fn parse_snapshot_should_return_snapshot_struct() {
        let snapshot = "boot@2020-08-12-1237-49-CHECKPOINT";
        let expected_snapshot = Snapshot::new(
            "boot",
            "boot",
            Local
                .datetime_from_str("2020-08-12-1237-49", SNAPSHOT_FORMAT)
                .unwrap(),
            "CHECKPOINT",
        );

        let result = parse_snapshot(&snapshot).unwrap();

        assert_eq!(expected_snapshot.pool(), result.pool());
        assert_eq!(expected_snapshot.dataset(), result.dataset());
        assert_eq!(expected_snapshot.date(), result.date());
        assert_eq!(expected_snapshot.label(), result.label());
    }

    #[test]
    fn get_snapshots_for_should_filter_correctly() {
        let initial_snapshots = vec![
            create_snapshot("boot", "2020-08-12-1237-49", "CHECKPOINT"),
            create_snapshot(
                "backup/tank/gentoo/home",
                "2020-07-13-2354-09",
                "CHECKPOINT",
            ),
            create_snapshot("tank/gentoo/os", "2020-07-13-2354-09", "CHECKPOINT"),
            create_snapshot("tank/gentoo/os", "2020-08-13-2354-09", "CHECKPOINT"),
            create_snapshot("tank/gentoo/os", "2020-08-13-2354-09", "LOL"),
        ];

        let expected_snapshots = vec![
            create_snapshot("tank/gentoo/os", "2020-07-13-2354-09", "CHECKPOINT"),
            create_snapshot("tank/gentoo/os", "2020-08-13-2354-09", "CHECKPOINT"),
        ];

        assert_eq!(
            expected_snapshots,
            get_snapshots_for("tank", "CHECKPOINT", initial_snapshots)
        );
    }

    #[test]
    fn all_snapshots_should_be_retrieved() {
        let expected_snapshots = vec![
            "boot@2020-08-12-1237-49-CHECKPOINT",
            "backup/tank/gentoo/home@2020-07-13-2354-09-CHECKPOINT",
            "tank/gentoo/os@2020-07-13-2354-09-CHECKPOINT",
            "tank/gentoo/os@2020-08-13-2354-09-CHECKPOINT",
        ];

        assert_eq!(expected_snapshots, get_snapshots(&FakeCommunicator));
    }

    #[test]
    fn get_excluded_snapshots_test() {
        let expected_snapshots = vec![create_snapshot("boot", "2020-08-12-1237-49", "CHECKPOINT")];

        assert_eq!(
            expected_snapshots,
            get_excluded_snapshots(
                &FakeCommunicator,
                &utility::get_fake_config("boot", "2020-05-01-1200-00", "")
            )
        );
    }

    #[test]
    fn get_relevant_snapshots_test() {
        pub struct FakeCommunicator;
        impl Communicator for FakeCommunicator {
            fn get_snapshots(&self) -> SystemResult {
                Ok("boot@2020-08-12-1237-49-CHECKPOINT\n\
                    tank/gentoo/os@2020-07-13-2354-09-CHECKPOINT\n\
                    tank/gentoo/os@2020-05-01-1100-00-CHECKPOINT\n\
                    tank/gentoo/home@2020-04-25-1300-15-CHECKPOINT\n\
                    tank@2020-01-01-2354-09-CHECKPOINT\n"
                    .to_string())
            }
            fn get_excluded_snapshots(&self, _exclude_file: &str) -> SystemResult {
                Ok("boot@2020-08-12-1237-49-CHECKPOINT\n\
                tank/gentoo/os@2020-07-13-2354-09-CHECKPOINT\n"
                    .to_string())
            }
        }

        let excluded_snapshots = vec![
            create_snapshot("tank/gentoo/home", "2020-04-25-1300-15", "CHECKPOINT"), // older but excluded
        ];
        let expected_snapshots = vec![
            create_snapshot("tank/gentoo/os", "2020-05-01-1100-00", "CHECKPOINT"),
            create_snapshot("tank", "2020-01-01-2354-09", "CHECKPOINT"),
        ];
        let relevant_snapshots = get_relevant_snapshots(
            &FakeCommunicator,
            &utility::get_fake_config("tank", "2020-05-01-1200-00", ""),
            &excluded_snapshots,
        );
        assert_eq!(expected_snapshots, relevant_snapshots);
    }

    #[test]
    fn remove_excluded_snapshots_test() {
        let snapshots = vec![
            create_snapshot("boot", "2020-08-12-1237-49", "CHECKPOINT"),
            create_snapshot("tank/gentoo/os", "2020-07-13-2354-09", "CHECKPOINT"),
            create_snapshot("tank/gentoo/os", "2020-05-01-1100-00", "CHECKPOINT"),
            create_snapshot("tank/gentoo/home", "2020-04-25-1300-15", "CHECKPOINT"),
            create_snapshot("tank", "2020-01-01-2354-09", "CHECKPOINT"),
        ];

        let excluded_snapshots = vec![
            create_snapshot("boot", "2020-08-12-1237-49", "CHECKPOINT"),
            create_snapshot("tank/gentoo/home", "2020-04-25-1300-15", "CHECKPOINT"),
            create_snapshot("tank", "2020-01-01-2354-09", "CHECKPOINT"),
        ];

        let expected_snapshots = vec![
            create_snapshot("tank/gentoo/os", "2020-07-13-2354-09", "CHECKPOINT"),
            create_snapshot("tank/gentoo/os", "2020-05-01-1100-00", "CHECKPOINT"),
        ];

        assert_eq!(
            expected_snapshots,
            remove_excluded_snapshots(snapshots, &excluded_snapshots)
        )
    }

    #[test]
    fn build_list_to_delete_test() {
        let snapshots = vec![
            create_snapshot("tank/gentoo/os", "2020-07-13-2354-09", "CHECKPOINT"),
            create_snapshot("tank/gentoo/os", "2020-05-01-1100-00", "CHECKPOINT"),
            create_snapshot("tank/gentoo/os", "2020-09-05-1300-00", "CHECKPOINT"),
        ];
        let references = snapshots.iter().collect();
        let expected_result = "tank/gentoo/os@2020-07-13-2354-09-CHECKPOINT,2020-05-01-1100-00-CHECKPOINT,2020-09-05-1300-00-CHECKPOINT";
        assert_eq!(expected_result, build_list_to_delete(&references));
    }

    #[test]
    fn get_datasets_test() {
        let snapshots = vec![
            create_snapshot("tank/gentoo/os", "2020-07-13-2354-09", "CHECKPOINT"),
            create_snapshot("tank/lol", "2020-05-01-1100-00", "CHECKPOINT"),
            create_snapshot("tank/home", "2020-09-05-1300-00", "CHECKPOINT"),
            create_snapshot("tank/home", "2020-09-05-1310-00", "CHECKPOINT"),
        ];

        let expected_result: HashSet<String> = vec![
            "tank/gentoo/os".to_string(),
            "tank/lol".to_string(),
            "tank/home".to_string(),
        ]
        .into_iter()
        .collect();

        assert_eq!(expected_result, get_datasets(&snapshots));
    }

    #[test]
    fn calculate_percentage_test() {
        let numerator = 1.0;
        let denominator = 10.0;
        let expected = numerator / denominator * 100.0;
        assert_eq!(expected, calculate_percentage(numerator, denominator));
    }

    #[test]
    fn destroy_snapshots_test() {
        let snapshots = vec![
            create_snapshot("tank/gentoo/os", "2020-07-13-2354-09", "CHECKPOINT"),
            create_snapshot("tank/lol", "2020-05-01-1100-00", "CHECKPOINT"),
            create_snapshot("tank/home", "2020-09-05-1300-00", "CHECKPOINT"),
            create_snapshot("tank/home", "2020-09-05-1310-00", "CHECKPOINT"),
        ];

        let mut expected_results: Vec<&Snapshot> = snapshots.iter().collect();
        let mut results = destroy_snapshots(&FakeCommunicator, &snapshots, 100);

        expected_results.sort();
        results.sort();
        assert_eq!(expected_results, results);
    }

    #[test]
    fn get_cutoff_date_should_default_to_30_days_ago() {
        let now = Local::now();
        let expected_date = now - Duration::days(30);
        let result = get_cutoff_date(now);
        assert_eq!(expected_date, result);
    }
}
