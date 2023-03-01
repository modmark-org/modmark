# **How to contribute**

We are using GitHub Issues and Pull Requests to structure and organize changes to the project. Pull requests are always welcome! Below are specific instructions for maintainers:

## GitHub workflow

### 1. Create Issue

The workflow pipeline starts with someone creating a issue, this could be for a feature request or for reporting a bug bug for example. This is done through [the issues page](https://github.com/modmark-org/modmark/issues). The templates are self-explanatory, and after this, the issue on the "markmod" project board below the "projects" header.

### 2. Create a branch

When you want to resolve an issue, you may create a branch by doing:

> git checkout -b [branch-name]

The branch should be created from the `main` branch, and should follow the following naming convention:

> name/GH-[issue-no]/[iteration-no]

Thus, the first iteration of issue nr 3 could look like:

> hugom/GH-3/0

### 3. Implement the fix in your branch

When you feel satisfied with the code written add the files you want to add to the codebase by using:

> git commit

Then write a commit message that references the original issue your solving, by prefixing the message with `GH-[issue-no]`. The message should briefly describe what the commit does, and it is important that this is written in **imperative form**. Thus, a commit message that fix issue 3 could look like this:

> GH-3: Fix and develop something

Note how this uses "fix" and not "fixes" in imperative form.

### 4. Push

When the code is committed, you need to push it to the main repository. To do this you have to connect your new branch to the upstream which is done by the following:

> git push --set-upstream origin [branch-name]

### 5. Make a Pull Request

To rebase your branch into `main`, open a pull request on [GitHub](https://github.com/modmark-org/modmark/pulls). To be able to rebase this, one other person has to do a code review approve it, and it has to pass all the tests. The pull request message should contain a description of what you have done, and must include the following text:

> Resolves: GH-[issue-no]
