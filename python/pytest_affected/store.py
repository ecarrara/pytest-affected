import json
import os
import pickle
import sqlite3
from dataclasses import dataclass
from typing import Any


@dataclass
class RunInstance:
    nodeid: str
    files: list[tuple[str, int]]
    reports: Any


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
            "create table if not exists instances (node_id text, run_at text default current_timestamp, files text, reports blob, primary key (node_id, files))"
        )

    def list_recent_runs(self, node_id: str, limit: int) -> list[RunInstance]:
        cursor = self._connection.cursor()
        cursor.execute(
            "select files, reports from instances where node_id = ? order by run_at desc limit 1",
            (node_id,),
        )
        runs = [
            RunInstance(
                nodeid=node_id,
                files=json.loads(row[0]),
                reports=pickle.loads(row[1]),
            )
            for row in cursor.fetchall()
        ]
        return runs

    def save_runs(self, runs: list[RunInstance], chunk_size=500):
        for i in range(0, len(runs), chunk_size):
            with self._connection:
                self._connection.executemany(
                    "insert into instances (node_id, files, reports) values (?, ?, ?) on conflict do nothing",
                    [
                        (
                            run.nodeid,
                            json.dumps(run.files),
                            pickle.dumps(run.reports, protocol=pickle.HIGHEST_PROTOCOL),
                        )
                        for run in runs[i : i + chunk_size]
                    ],
                )
