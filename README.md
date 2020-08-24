## Honeydew - v0.8.0
##### Jonathan Vasquez (fearedbliss)

## Description

A simple snapshot cleaner for ZFS.

## Usage

To start using the application, all you need to do is run:

**`./honeydew -p <pool name>`**

By default, the cut off point for what snapshots are considered old will
default to **`30`** days before today's date. You can use many of the options
documented below to modify the behavior (Including the ability to list
snapshots that should be excluded (and thus protected) from deletion).
For more information, you can run: **`./honeydew -h`** for a detailed list of
parameters as well.

For example, if we wanted to clean a pool called **`tank`**, use an exclude
file called **`excluded_snapshots`** that contains a list of snapshots to
exclude (one per line), we want to show what snapshots will be removed, and
what snapshots are excluded, and we want to set an arbitrary date in the
future that will make all of our snapshots old (and thus you are basically
saying: delete all the snapshots except the ones I've excluded), you can do
so as follows:

**`./honeydew -p tank -e excluded_snapshots -s -x -d 2099-01-01-0000-00`**

If you wanted to only remove snapshots that have a particular tag, you can
use the **`-l`** option. For example, the following command will also only
delete snapshots that have the **`ANIMALS`** tag:

**`./honeydew -p tank -e excluded_snapshots -s -x -d 2099-01-01-0000-00 -l ANIMALS`**

## Format

For simplicity, there is only one snapshot format allowed/accepted, which is in
the following format:

**`YYYY-mm-dd-HHMM-ss-LABEL`** => **`2020-05-01-2345-15-CHECKPOINT`**

The following command will yield a correctly formatted date (GNU coreutils):

**`date +%F-%H%M-%S`**

You can then concatenate the label.

Example:

```
POOL="tank"
DATE=$(date +%F-%H%M-%S)
TAG="ANIMALS"
SNAPSHOT_NAME="${DATE}-${TAG}"

zfs snapshot ${POOL}@${SNAPSHOT_NAME}
```

The above should yield a snapshot similar to the following:

**`tank@2020-08-23-1023-17-ANIMALS`**

## Options

```
USAGE:
    honeydew [FLAGS] [OPTIONS] --pool <pool>

FLAGS:
    -n, --dry-run          Performs a dry run. No deletions will occur.
    -h, --help             Prints help information
    -f, --no-confirm       Deletes snapshots without confirmation. Used primarily for cron.
    -c, --show-config      Displays the full configuration options used by the application.
    -x, --show-excluded    Show snapshots that will be excluded.
    -s, --show-queued      Show snapshots that will be removed.
    -V, --version          Prints version information

OPTIONS:
    -d, --date <date>                      The slice date that you want to use as your end point for snapshot deletions.
    -e, --exclude-file <exclude-file>      Excludes the list of snapshots in this file (one snapshot per line).
    -l, --label <label>                    The label of the snapshots that should be cleaned.
    -i, --per-iteration <per-iteration>    Number of snapshots to delete per iteration.
    -p, --pool <pool>                      The pool you want to clean.
```
                        
## Build

The easiest way to build the project is to have **`cargo`** installed and run:
**`cargo build --release`**.

If you wish not to build the project yourself, you can use a pre-built one I've
included at **`target/release/honeydew`**.

## License

**`Apache License 2.0`**

## Dependencies

- **`ZFS`** must be installed on your system and available in your PATH.

## Warnings

#### `-i, --per-iteration`

The default amount of snapshots that will be batched for deletion and passed to
ZFS is **`100`** at a time. The reason for this is that I've experienced full
system lockups many years ago when attempting to pass thousands of snapshots at
a time to ZFS (While my system was running Gentoo Linux installed on ZFS).
This may have been due to the previous Python 3 implementation of this program
that used **`shell=True`** to run the ZFS command (Which would spawn a separate
process through the shell, and would also have to take the shell's ARG_MAX
limit into account). Although I highly doubt it. Thus I believe this is an
issue with ZFS and/or Linux at the time which may or may not still exist.
Regardless, take care when increasing the amount of snapshots to delete per
round. The lower the batch, the more stable it will be.

## Contributions

Before opening a PR, please make sure the code is properly formatted and all
tests are passing. You can do this by running: **`cargo fmt`** and
**`cargo test`** respectively.
