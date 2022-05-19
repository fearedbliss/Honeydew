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

pub mod utility {
    use super::super::*;
    pub struct FakeCommunicator {
        does_file_exist: bool,
    }
    impl FakeCommunicator {
        pub fn new(does_file_exist: bool) -> FakeCommunicator {
            FakeCommunicator { does_file_exist }
        }
    }
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
        fn does_file_exist(&self, _filename: &str) -> bool {
            self.does_file_exist
        }
    }

    pub fn get_fake_config(pool: &str, date: &str, label: &str) -> Config {
        Config::new(
            &FakeCommunicator::new(true),
            pool,
            date,
            "",
            false,
            false,
            false,
            100,
            true,
            label,
            false,
        )
    }

    pub fn create_snapshot(dataset: &str, time: &str, label: &str) -> Snapshot {
        let splinters: Vec<_> = dataset.split("/").collect();
        let pool = splinters[0];
        let date = Local.datetime_from_str(time, SNAPSHOT_FORMAT).unwrap();

        Snapshot::new(pool, dataset, date, label)
    }
}
