# Repository Setup Guide

This document outlines the recommended repository settings and permissions for the Wole project to ensure proper contribution workflow and code quality.

## Branch Protection Rules

### Main/Master Branch Protection

**Location:** Repository Settings → Branches → Branch protection rules

**Recommended Settings:**

1. **Require a pull request before merging**
   - ✅ Require approvals: **1** (at least one reviewer)
   - ✅ Dismiss stale pull request approvals when new commits are pushed
   - ✅ Require review from Code Owners (if CODEOWNERS file exists)

2. **Require status checks to pass before merging**
   - ✅ Require branches to be up to date before merging
   - Required status checks:
     - `check` (Code Quality)
     - `test` (Run Tests)

3. **Require conversation resolution before merging**
   - ✅ Require all conversations on code to be resolved

4. **Restrict who can push to matching branches**
   - ✅ Do not allow bypassing the above settings (even for admins)

5. **Allow force pushes**
   - ❌ Do not allow force pushes

6. **Allow deletions**
   - ❌ Do not allow branch deletion

## Repository Permissions

### Collaborator Access

**Location:** Repository Settings → Collaborators

- **Write access**: Grant to trusted contributors who can review and merge PRs
- **Read access**: Default for all contributors (via forks)

### Actions Permissions

**Location:** Repository Settings → Actions → General

- ✅ Allow all actions and reusable workflows
- ✅ Allow actions created by GitHub
- ✅ Allow actions by Marketplace verified creators
- ✅ Allow local actions and reusable workflows

### Issues and Pull Requests

**Location:** Repository Settings → General

- ✅ Issues: Enabled
- ✅ Projects: Enabled (optional)
- ✅ Wiki: Disabled (using README/docs instead)
- ✅ Discussions: Enabled (optional, for community Q&A)

### Merge Options

**Location:** Repository Settings → General → Pull Requests

- ✅ Allow merge commits
- ✅ Allow squash merging (recommended for cleaner history)
- ✅ Allow rebase merging
- ✅ Automatically delete head branches (recommended)

## Labels

**Location:** Issues → Labels

Recommended labels:

**Type:**
- `bug` - Something isn't working
- `enhancement` - New feature or request
- `documentation` - Documentation improvements
- `question` - Further information is requested

**Priority:**
- `priority: high` - Urgent issues
- `priority: medium` - Normal priority
- `priority: low` - Nice to have

**Status:**
- `good first issue` - Good for newcomers
- `help wanted` - Extra attention is needed
- `wontfix` - This will not be worked on

**Area:**
- `area: tui` - Terminal UI related
- `area: cli` - Command-line interface
- `area: core` - Core functionality
- `area: build` - Build system
- `area: windows` - Windows-specific

## Code Owners

Create `.github/CODEOWNERS` file to automatically request reviews from specific maintainers for certain paths:

```
# Global owners
* @jplx05

# Core functionality
/src/ @jplx05

# CI/CD
/.github/workflows/ @jplx05
```

## Security Settings

**Location:** Repository Settings → Security

1. **Dependabot alerts**: ✅ Enabled
2. **Dependabot security updates**: ✅ Enabled
3. **Secret scanning**: ✅ Enabled (if available)
4. **Code scanning**: Consider enabling (requires GitHub Advanced Security)

## Webhooks (Optional)

For integration with external services:
- CI/CD systems
- Project management tools
- Chat notifications

## Next Steps

1. Configure branch protection for `master` branch (or `main` if you switch)
2. Set up repository permissions
3. Create labels as needed
4. Add CODEOWNERS file if you have multiple maintainers
5. Enable security features
6. Review and adjust Actions permissions

## Notes

- These settings ensure code quality and maintainability
- Branch protection prevents direct pushes to main branch
- Required reviews ensure code is reviewed before merging
- Status checks ensure tests pass before merging
