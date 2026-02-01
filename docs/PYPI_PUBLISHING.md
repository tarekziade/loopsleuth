# Publishing LoopSleuth to PyPI

This guide explains how to publish LoopSleuth to PyPI using GitHub Actions with trusted publishing.

## Overview

The publishing workflow (`.github/workflows/publish.yml`) automatically:
1. Builds wheels for Linux, macOS (Intel & Apple Silicon), and Windows
2. Builds a source distribution (sdist)
3. Publishes to PyPI using trusted publishing (no API tokens needed!)

## One-Time Setup

### 1. Create PyPI Account

If you don't have a PyPI account:
1. Go to https://pypi.org/account/register/
2. Verify your email address

### 2. Configure Trusted Publishing on PyPI

Trusted publishing allows GitHub Actions to publish directly without storing API tokens.

1. **Go to PyPI**: https://pypi.org/manage/account/publishing/
2. **Add a new publisher**:
   - **PyPI Project Name**: `loopsleuth` (must match the name in `pyproject.toml`)
   - **Owner**: Your GitHub username or organization
   - **Repository name**: `loopsleuth`
   - **Workflow name**: `publish.yml`
   - **Environment name**: `pypi`
3. **Save**

**Important**: For the first release, you need to create the project on PyPI first:
- Option A: Do a manual upload once (see "Manual Publishing" below)
- Option B: Use Test PyPI first, then switch to regular PyPI

### 3. Create GitHub Environment

1. Go to your GitHub repository settings
2. Navigate to **Environments**
3. Click **New environment**
4. Name it `pypi` (must match the workflow)
5. (Optional) Add protection rules:
   - Required reviewers
   - Deployment branches (e.g., only `main`)

## Publishing Methods

### Method 1: Automatic on Release (Recommended)

1. **Update version** in `pyproject.toml` and `python/loopsleuth/__init__.py`:
   ```toml
   version = "0.1.0"
   ```

2. **Commit and push**:
   ```bash
   git add pyproject.toml python/loopsleuth/__init__.py
   git commit -m "Bump version to 0.1.0"
   git push
   ```

3. **Create a GitHub release**:
   ```bash
   # Create a tag
   git tag v0.1.0
   git push origin v0.1.0

   # Or create a release through GitHub UI
   # Go to: https://github.com/yourusername/loopsleuth/releases/new
   # - Tag: v0.1.0
   # - Title: v0.1.0
   # - Description: Release notes
   # - Click "Publish release"
   ```

4. **GitHub Actions will automatically**:
   - Trigger the `publish.yml` workflow
   - Build wheels for all platforms
   - Build source distribution
   - Publish to PyPI

5. **Monitor the workflow**:
   - Go to: https://github.com/yourusername/loopsleuth/actions
   - Check the "Build and Publish to PyPI" workflow

### Method 2: Manual Trigger

You can manually trigger the workflow from GitHub Actions:

1. Go to: https://github.com/yourusername/loopsleuth/actions
2. Select "Build and Publish to PyPI" workflow
3. Click "Run workflow"
4. Choose:
   - **Branch**: Usually `main`
   - **Publish to Test PyPI**: Check to test first, uncheck for production
5. Click "Run workflow"

### Method 3: Test PyPI First (Recommended for First Release)

Test your package on Test PyPI before publishing to production:

1. **Configure Test PyPI trusted publishing**:
   - Go to: https://test.pypi.org/manage/account/publishing/
   - Add publisher (same settings as PyPI)

2. **Manually trigger workflow**:
   - Check "Publish to Test PyPI" option

3. **Test installation**:
   ```bash
   pip install --index-url https://test.pypi.org/simple/ --extra-index-url https://pypi.org/simple loopsleuth
   ```

4. **If successful, publish to production PyPI**:
   - Create a GitHub release or manually trigger without Test PyPI option

## Manual Publishing (Without GitHub Actions)

If you need to publish manually:

### Prerequisites
```bash
pip install build twine
```

### Build
```bash
# Clean previous builds
rm -rf build dist *.egg-info

# Build source distribution and wheel
python -m build

# Check the built packages
ls -lh dist/
```

### Upload to Test PyPI
```bash
twine upload --repository testpypi dist/*
```

### Upload to PyPI
```bash
twine upload dist/*
```

You'll be prompted for your PyPI username and password (or API token).

## Workflow Details

The `.github/workflows/publish.yml` workflow has three jobs:

### 1. build-wheels
- **Runs on**: Ubuntu, macOS (Intel & ARM), Windows
- **Builds**: Python wheels for Python 3.8-3.12
- **Uses**: cibuildwheel for cross-platform builds
- **Installs**: Rust toolchain and CMake in each environment
- **Output**: Platform-specific wheels (`.whl` files)

### 2. build-sdist
- **Runs on**: Ubuntu
- **Builds**: Source distribution (`.tar.gz`)
- **Output**: Source package with all source code

### 3. publish
- **Runs after**: Both build jobs complete
- **Uses**: Trusted publishing (OIDC authentication)
- **Publishes**: All wheels and sdist to PyPI
- **Requires**: GitHub environment `pypi` with proper permissions

## Platform Coverage

The workflow builds wheels for:

| Platform | Architectures | Python Versions |
|----------|--------------|-----------------|
| Linux | x86_64 | 3.8, 3.9, 3.10, 3.11, 3.12 |
| macOS | x86_64 (Intel) | 3.8, 3.9, 3.10, 3.11, 3.12 |
| macOS | arm64 (Apple Silicon) | 3.8, 3.9, 3.10, 3.11, 3.12 |
| Windows | x86_64 | 3.8, 3.9, 3.10, 3.11, 3.12 |

This means users can `pip install loopsleuth` without needing Rust installed!

## Version Management

**Important**: Keep versions synchronized:

1. **pyproject.toml**: `version = "0.1.0"`
2. **python/loopsleuth/__init__.py**: `__version__ = "0.1.0"`
3. **Git tag**: `v0.1.0`

Consider using a version management tool like `bump2version` or `commitizen` to automate this.

## Checklist for Each Release

- [ ] Update version in `pyproject.toml`
- [ ] Update version in `python/loopsleuth/__init__.py`
- [ ] Update CHANGELOG.md (if you have one)
- [ ] Commit changes: `git commit -m "Bump version to X.Y.Z"`
- [ ] Push to GitHub: `git push`
- [ ] Create git tag: `git tag vX.Y.Z`
- [ ] Push tag: `git push origin vX.Y.Z`
- [ ] Create GitHub release with release notes
- [ ] Wait for workflow to complete
- [ ] Verify on PyPI: https://pypi.org/project/loopsleuth/
- [ ] Test installation: `pip install loopsleuth`
- [ ] Test functionality: `loopsleuth --help`

## Troubleshooting

### "Project name not found on PyPI"
**Solution**: For first release, either:
- Do a manual upload first with `twine upload`
- Or use Test PyPI first, then create the project on PyPI

### "Trusted publishing failed"
**Solution**: Check that:
- Environment name is exactly `pypi` in workflow and GitHub settings
- Repository owner and name are correct in PyPI settings
- Workflow name is `publish.yml` in PyPI settings

### "Build failed on a platform"
**Solution**:
- Check the GitHub Actions logs
- Common issues: Missing CMake, Rust not installed, llama.cpp build errors
- Test locally with: `pip install cibuildwheel && cibuildwheel --platform linux`

### "Wheel doesn't work on user's system"
**Solution**:
- Check that the platform is covered in the build matrix
- User may need to install from source if platform not supported
- Consider adding more platforms to the matrix

### "Version already exists on PyPI"
**Solution**:
- You cannot overwrite versions on PyPI
- Increment version number and publish again

## Security Notes

- **Never commit API tokens**: Use trusted publishing instead
- **Protect the `pypi` environment**: Require approvals for deployments
- **Review workflow changes**: Changes to `.github/workflows/publish.yml` should be carefully reviewed
- **Use signed releases**: Consider signing your releases with GPG

## Resources

- [PyPI Trusted Publishing Guide](https://docs.pypi.org/trusted-publishers/)
- [cibuildwheel Documentation](https://cibuildwheel.readthedocs.io/)
- [setuptools-rust Documentation](https://setuptools-rust.readthedocs.io/)
- [Python Packaging Guide](https://packaging.python.org/)

## After Publishing

Once published, users can install with:

```bash
pip install loopsleuth
```

Update your README to reflect this!

## Publishing Checklist (Quick Reference)

```bash
# 1. Update versions
vim pyproject.toml  # Update version
vim python/loopsleuth/__init__.py  # Update __version__

# 2. Commit and tag
git add pyproject.toml python/loopsleuth/__init__.py
git commit -m "Release v0.1.0"
git tag v0.1.0
git push origin main v0.1.0

# 3. Create GitHub release
# Go to: https://github.com/yourusername/loopsleuth/releases/new
# Or use GitHub CLI:
gh release create v0.1.0 --title "v0.1.0" --notes "Release notes here"

# 4. Wait for GitHub Actions to complete
# 5. Verify: pip install loopsleuth
```
