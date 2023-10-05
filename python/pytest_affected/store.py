import json
import os
import pickle
import sqlite3
from dataclasses import dataclass, field
from typing import Any


@dataclass
class RunInstance:
    nodeid: str
    files: list[tuple[str, int]]
    reports: Any
    setup_duration: float
    call_duration: float
    teardown_duration: float
    failed : bool = field(default=False)


class Store:
    def list_recent_runs(self, node_id: str, limit: int) -> list[RunInstance]:
        raise NotImplementedError

    def save_runs(self, runs: list[RunInstance]):
        raise NotImplementedError


class SQLiteStore(Store):
    def __init__(self):
        os.makedirs(".pytest_affected", exist_ok=True)
        self._connection = sqlite3.connect(".pytest_affected/db.sqlite3")
        self._connection.execute(
            "create table if not exists instances (node_id text primary key, run_at text default current_timestamp, files text, setup_duration real, call_duration real, teardown_duration real, reports blob)"
        )
        self._connection.execute("pragma cache_size = -100000")

    def list_recent_runs(self, node_id: str, limit: int) -> list[RunInstance]:
        cursor = self._connection.cursor()
        cursor.execute(
            "select files, reports, setup_duration, call_duration, teardown_duration from instances where node_id = ? order by run_at desc limit 1",
            (node_id,),
        )
        runs = [
            RunInstance(
                nodeid=node_id,
                files=json.loads(row[0]),
                reports=pickle.loads(row[1]),
                setup_duration=row[2],
                call_duration=row[3],
                teardown_duration=row[4],
            )
            for row in cursor.fetchall()
        ]
        return runs

    def save_runs(self, runs: list[RunInstance], chunk_size=500):
        for i in range(0, len(runs), chunk_size):
            with self._connection:
                self._connection.executemany(
                    "insert into instances (node_id, files, setup_duration, call_duration, teardown_duration, reports) values (?, ?, ?, ?, ?, ?) on conflict do nothing",
                    [
                        (
                            run.nodeid,
                            json.dumps(run.files),
                            pickle.dumps(run.reports, protocol=pickle.HIGHEST_PROTOCOL),
                            run.setup_duration,
                            run.call_duration,
                            run.teardown_duration,
                        )
                        for run in runs[i : i + chunk_size]
                    ],
                )

    def delete_old_runs(self, secs):
        self._connection.execute(f"delete from instances where run_at < datetime(current_timestamp, '-{secs} seconds')")
