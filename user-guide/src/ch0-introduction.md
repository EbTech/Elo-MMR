# Introduction

Welcome to the User's Guide for the Elo-MMR and `multi-skill`.

Elo-MMR is a skill estimation mechanism or, in more common terms, a *rating system*. It's a natural generalization of Elo and Glicko to the case of games with more than 2, and potentially thousands, of simultaneous players that are ranked against one another. For full technical details, see the [paper we published in WWW 2021](https://arxiv.org/abs/2101.00400). In this guide, we go over some of the more practical considerations for a user who wishes to rate players of some game.

`multi-skill` is the name of the research software package built for testing Elo-MMR and other candidate rating systems on a variety of datasets. It was used to investigate and present the results in the paper, and we hope it will be useful for further experimentation and practical applications.

This book can be built using `mdbook`, or its markdown pages manually browsed from the [summary listing](SUMMARY.md).