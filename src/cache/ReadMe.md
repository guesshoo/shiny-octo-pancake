# Quick guide to using FlatBuffers

*  Please see https://flatbuffers.dev/schema/ for guide on FlatBuffers schema.

## 1. Install FlatBuffers Compiler (flatc)
### Option 1: Build from source
```bash
git clone https://github.com/google/flatbuffers.git
cd flatbuffers
cmake -G "Unix Makefiles"
make

# Then copy `flatc` to directory that is include in your PATH env variable.
```

### Option 2: Look for it in Artifactory.

* Look it up

### 2. Define your schema and generate rust code
* From where your `myschema.fbs` lives, run
```bash
flatc --rust -o src/ myschema.fbs
```
* --rust tells flatc to emit Rust code.

* -o src/ places the generated .rs file(s) into your src/ folder.

