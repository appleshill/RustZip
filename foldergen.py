import os
import random

# Config
BASE_DIR = "test_data"
NUM_SUBDIRS = 5       # how many subdirectories
FILES_PER_SUBDIR = 10 # how many files per subdirectory
FILE_SIZE_KB = 200    # size of each file in KB

os.makedirs(BASE_DIR, exist_ok=True)

for subdir_idx in range(NUM_SUBDIRS):
    subdir_path = os.path.join(BASE_DIR, f"folder_{subdir_idx}")
    os.makedirs(subdir_path, exist_ok=True)

    for file_idx in range(FILES_PER_SUBDIR):
        file_path = os.path.join(subdir_path, f"file_{file_idx}.bin")
        with open(file_path, "wb") as f:
            # Write random bytes
            f.write(os.urandom(FILE_SIZE_KB * 1024))

print(f"✅ Created {NUM_SUBDIRS * FILES_PER_SUBDIR} files in '{BASE_DIR}'")