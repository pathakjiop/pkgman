## Contributing

Contributions are very welcome. To keep things clean and mergeable, please follow these conventions.

### Fork & Branch

```
# fork first
git checkout <existing>
git checkout -b <new>
git add <file(s)>
git commit 
# describe what this commit fixes, ideally one fix per commit
git push
```

### Workflow

```bash
# 1. Fork the repo and clone your fork
git clone https://github.com/pathakjiop/pkgman.git
cd pkgman

# 2. Create your branch from main
git checkout -b feat/your-feature-name

# 3. Make your changes, commit with a clear message
git commit -m "feat: add catppuccin theme support"

# 4. Push and open a PR against main
git push origin feat/your-feature-name
```

### Checks

```
cargo fmt --check
cargo clippy
```

### Commit message format

Follow the conventional commit style:

```
<type>(subject): <short imperative description>

Optional longer body explaining the why/how, not the what.
```

### PR checklist

Before opening a PR, make sure:

- [ ] `cargo build --release` succeeds with no warnings
- [ ] Your branch name follows the naming convention above
- [ ] Changes are scoped: one concern per PR
- [ ] You've updated documentation if behavior changed
- [ ] For new features, a brief description is in the PR body
- [ ] For issues, linking to existing #XXXX is useful too

---
