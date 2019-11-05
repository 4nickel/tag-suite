# Tag & Tdb
###### A file-tagging and -indexing suite using extended filesystem attributes
&nbsp;
> This is work in progress. Most things work, but some features are not yet implemented and it's pretty rough around the edges. Documentation is also lacking :(

&nbsp;
#### Overriding Designs
- You can tag any filesystem object that supports xattrs.
- Integrates nicely with other command line programs and shell pipelines.
- Simple universal formats and encodings - no lock in.

&nbsp;
#### Features
- Custom querying and filtering syntax helps you find whatever you are looking for.
- Save and compose queries - stay DRY.
- Symlink the results of your queries and view them in your file browser.
- Keep your metadata tidy and neat: establish *conventions* and *enforce* them.

&nbsp;
Tags are stored as UTF-8 encoded text in extended filesystem attributes supported by many open-source filesystems. Any valid UTF-8 string is a valid tag, however, tags may not contain any commas, as they are used as seperators internally, or begin with, or end on a colon.

##### Let's get started!
&nbsp;
To get an overview run:
```sh
$ tag --help
$ tdb --help
```
Now, add a tag to some files:
```sh
$ cd /bakery
$ tag add 'Delicious Dough' Muffins.jpg ApplePie.png
```
Great! Let's build the index..
```sh
$ tdb update Muffins.jpg ApplePie.png
```
.. and run a query:
```sh
$ tdb query '=[Delicious Dough]'
/bakery/Muffins.jpg
/bakery/ApplePie.png
```
### Queries
###### So, what's going on with those brackets and stuff?

To make finding things easy, tdb features a simple but convenient syntax for querying files from the database and filtering the results. The above could be read as: *go find me all files that have a tag that matches ```Delicious Dough```*.

Every expression consists of a *modifier*, in this case the *match* modifier ```=``` and a *body*, the part inside the square brackets.

> As the match modifier is the most commonly used one, it is implied if no other modifier is specified:
```tdb query '[FancyPants]'``` is equivalent to ```tdb query '=[FancyPants]'```.

Currently, there are two more modifiers: the *comparison* modifier ```?``` and the *shell* modifier ```$```. While *query expressions* may only use the match modifier, *filter expressions* support all three.

Expressions can be combined in simple boolean logic using the *operators* ```&```, ```|``` and ```!```, which correspond to the logical *AND*, *OR* and *NOT* operations, respectively. Expressions can be grouped using round brackets. Consider the examples below to get a feeling for the syntax.

Let's find some files containing Garfield..
```sh
$ tdb query '[Meow] & [Lasagna]'
```
.. or how about good ol' Scoobers:
```sh
$ tdb query '([Doggo] | [K9]) & ![2Spooky4Scooby]'
```
Sometimes, it is useful to match things using a *wildcard*. Instead of writing..
```sh
$ tdb query '[Mashed Taters] | [Fried Taters] | [Sweet Taters]'
```
.. it is much more convenient to just:
```sh
$ tdb query '[%Taters]'
```
> If this reminds you of SQL's ```WHERE .. LIKE ..``` syntax, then you have a pretty good idea what's going on under the hood!

### Filters
In case wildcards are too limited, you can additionally *filter* the results of a query and use regex instead. Filter expressions may be slower, but are more flexible than query expressions.
```sh
$ tdb query '[CarpetBarf]' --filter '[(Mr ?)?Mittens]'
```
We can also use the other aforementioned modifiers. The following example yields only matching files with more than 4 tags attached, by using the comparison modifier:
```sh
$ tdb query '[Bathtub] & [Kayaking]' --filter '?[tags.len > 4]'
```
>The shell modifier ```$``` is super-duper slow, brittle, potentially dangerous and will be reworked soon. I don't recommend using it atm and I'm not gonna tell you how to do so :)

### Pipes
If even filters are not enough, tdb provides the --pipe <sh> option. As the name suggests, it pipes the results of a query through a shell script, allowing you to filter the input in any arbitrary way. Filenames are then simply read back from stdout of your script.

> Please be very careful about the scripts you use in pipes. While this is not inherently more dangerous than any ordinary shell pipeline, the fact that you are processing a potentially large amount of files gives you ample opportunity to clobber a potentially large amount of files! There is nothing tdb can do to protect you from gunning your foot by mistake.

### Namespaces
Until now, all examples only matched tags - but what if we want to match e.g. filenames? In order to let you filter and match other data stored in the database, tdb uses reserved *namespaces* under which we export data as pseudo-tags. The namespacing operator ```::``` is used to seperate namespaces. Currently, ```tdb```, ```path```, and ```kind``` are reserved. Below are some examples, to illustrate the concept.

Files with a ```.txt``` file extension:
```sh
$ tdb query '[kind::file] & [path::%.txt]'
```

Only directories:
```sh
$ tdb query '[kind::dir]'
```

Namespaces can be nested arbitrarily deeply. Of course, you don't have to use this feature at all in your tags, if you don't like it. Personally, I find it quite useful in order to group related tags.

>There is even some shorthand syntax to save you a couple keystrokes, note the leading/trailing colon:
```[Root:]``` and ```[:Leaf]``` expand to ```[Root::%]``` and ```[%::Leaf]```, repectively.

### Map
For convenience, tdb comes with a couple of pre-defined actions which you can map over the result of a query. The general syntax is:

```sh
$ tdb query <query> map <action> <args>
```
TODO: needs more documentation!

### Configuration
TODO: document this!
