# **How to contribute**

## Git workflow

<br />

### **Create issue**

The workflow pipeline starts with someone creating a issue, this could be a feature request or a bug for example. This is done through [the issues page](https://github.com/modmark-org/modmark/issues). The templates are self explanatory, and after this mark the issue on the "markmod" project board below the "projects" header.

<br />

### **Create branch**

> git checkout -b [branch-name]

To start working on the issue create a branch from main. This should be named using the following naming convention:

> name/GH-[issuenr]/iteration

Thus, the first iteration of issue nr 3 could look like:

> hugom/GH-3/0

<br />

### **Commit**

When you feel satisfied with the code written add the files you want to add to the codebase by using:

> git commit

Then write a commit-message that references the original issue your solving. The following text should describe what the commit does, its important this is written in **IMPERATIVE FORM**. Thus, a commit message that fix issue 3 would look like this:

> GH-3: Fix and develop something

Note how this uses "fix" and not "fixes"

<br />

### **Push**

Now that the code is commited you need to push it to the main repository. To do this you have to connect your new branch to the upstream which is done by the following:

> git push --set-upstream origin [branch-name]

 <br />

### **Pull request**

To lastly rebase this unto main, open a pull request on [github](www.github.com). To be able to rebase this one other person has to approve and it must pass all the test without merge conflicts. The pull request message should be structured as following:

> \- Resolves: GH-[issuenr]
