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

pub mod utility {
    use super::super::*;
    pub struct FakeCommunicator;
    impl Communicator for FakeCommunicator {
        fn get_snapshots(&self) -> SystemResult {
            Ok("boot@2020-08-12-1237-49-CHECKPOINT\n\
                backup/tank/gentoo/home@2020-07-13-2354-09-CHECKPOINT\n\
                tank/gentoo/os@2020-07-13-2354-09-CHECKPOINT\n\
                tank/gentoo/os@2020-08-13-2354-09-CHECKPOINT\n"
                .to_string())
        }
        fn destroy_snapshots(&self, snapshots: String) -> SystemResult {
            Ok(snapshots)
        }
        fn get_excluded_snapshots(&self, _exclude_file: &str) -> SystemResult {
            Ok("boot@2020-08-12-1237-49-CHECKPOINT\n\
            tank/gentoo/os@2020-07-13-2354-09-CHECKPOINT\n"
                .to_string())
        }
    }

    pub fn get_fake_config(pool: &str, date: &str, label: &str) -> Config {
        Config::new(pool, date, "", false, false, false, 100, true, label, false)
    }

    pub fn create_snapshot(dataset: &str, time: &str, label: &str) -> Snapshot {
        let splinters: Vec<_> = dataset.split("/").collect();
        let pool = splinters[0];
        let date = Local.datetime_from_str(time, SNAPSHOT_FORMAT).unwrap();

        Snapshot::new(pool, dataset, date, label)
    }
}
