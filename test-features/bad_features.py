#!/usr/bin/env python3

import os

import numpy as np
from sklearn.ensemble import RandomForestClassifier


DATA_PATH = '.'


def load_field(field):
    path_dat = os.path.join(DATA_PATH, f'feature_{field}.dat')
    path_name = os.path.join(DATA_PATH, f'feature_{field}.name')

    with open(path_name) as fh:
        names = fh.read().split()

    data = np.memmap(path_dat, mode='r', dtype=np.float32).reshape(-1, len(names))

    return data, names


def main():
    field0, names0 = load_field(795)
    field1, names1 = load_field(796)
    assert names0 == names1
    x = np.vstack([field0, field1])
    y = np.r_[np.zeros(field0.shape[0]), np.ones(field1.shape[0])]

    rfc = RandomForestClassifier(
        n_estimators=100,
        max_depth=int(np.log2(y.size)),
        n_jobs=-1,
    )
    print('Fitting')
    rfc.fit(x, y)
    print('Calculating score')
    score = rfc.score(x, y)
    print(score)


if __name__ == '__main__':
    main()
