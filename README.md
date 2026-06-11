## bumahhfiles

temporary file host

sister project of [dumahhfiles](https://github.com/prashantrahul141/dumahhfiles)

### Configuration

Configuration can be done via environment variables
| name                       | default            | purpose                       |
|----------------------------|--------------------|-------------------------------|
| BUMAHH_ROOT_DIR            | "files"            | where to store files          |
| BUMAHH_MAGIC_KEY           | "magic-key"        | used to compute delete keys   |
| BUMAHH_INTERNAL_HOST       | "0.0.0.0"          | where to listen               |
| BUMAHH_INTERNAL_PORT       | 3000               | which port to listen          |
| BUMAHH_EXTERNAL_PROTOCOL   | "http"             | used to format links          |
| BUMAHH_EXTERNAL_HOST       | "0.0.0.0:3000"     | used to format links          |
| BUMAHH_GC_INTERVAL_MIN     | 30                 | garbage collection interval   |
| BUMAHH_MAX_FILE_COUNT      | 5                  | max file upload count per req |
| BUMAHH_MAX_FILENAME_LENGTH | 240                | max filename in storage       |
| BUMAHH_MAX_ON_DISK_STORAGE | 16106127360 (15GB) | max storage allowed           |
| BUMAHH_MAX_FILE_SIZE       | 209715200 (200MB)  | max per file size             |
| BUMAHH_MIN_RETENTION_HRS   | 1 hour             | Min retention time in hours   |
| BUMAHH_MAX_RETENTION_HRS   | 168 hours          | Max retention time in hours   |
| RUST_LOG                   | "debug"            | logging level                 |
| version                    | "unknown"          | current commit hash           |


### Retention

Files are deleted automatically depending on the size of the file using the
following equation:
```
max_time = 7 * 24 hrs
min_time = 1 hr
max_size = 200 MB
time = min_time + max_time * (1 - file_size / max_size) ^ e
```
here's a pretty graph of the above equation:
```

  168 +.
  hrs |..
      | ...
      |   ..
      |     ..
      |      ...
      |        ...
t     |          ..
i     |            ..
m  84 |              ...
e     |                ...
      |                  ...
      |                     ...
      |                        ...
      |                           ....
      |                              .....
      |                                  ......
      |                                       ..............
      |                                                    ................
    0 +----------------------------------------------------------------------+
      0                                  100                             200MB
                                       size (MB)
```
