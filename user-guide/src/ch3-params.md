# Elo-MMR Parameters

In the paper, the Elo-MMR algorithm is specified in terms of five parameters:

- \\(\rho\in[0,\infty]\\), the inverse momentum,

- \\(\mu_0\in(-\infty,\infty)\\), the starting skill mean,

- \\(\sigma_0\in(0,\infty)\\), the starting skill deviation,

- \\(\beta\in(0,\infty)\\), the performance deviation,

- \\(\gamma\in(0,\infty)\\), the temporal diffusion.

The parameter \\(\rho\\) is specific to the logistic performance model. Roughly speaking, \\(1/\rho\\) corresponds to the amount by which volatile players may be "boosted" when their skill level changes suddenly. If in doubt, \\(\rho=1\\) is a reasonable setting.

Linear transformations of the entire rating system would leave the relative ranking of players invariant. Thus, \\(\mu_0\\) is arbitrary, and the deviations \\(\sigma_0,\beta,\gamma\\) can be multiplied by an arbitrary common constant. We default to Mark Glickman's convention of setting the scale by fixing \\(\mu_0,\sigma_0 = 1500,350\\). An alternative is the TrueSkill convention \\(\mu_0,\sigma_0 = 25,8.333\\). TrueSkill conservatively reports a player's public rating as \\(\mu-3\sigma\\), which under this setting would start at zero.

We're left with only two interesting parameters, \\(\beta,\gamma\\), whose setting should depend on domain-specific considerations, and may even differ between contests. To make the process more user-friendly, our implementation does not set these parameters directly. Instead, they're derived from three user-specified parameters:

- \\(\beta_1\in(0,\infty)\\), the default performance deviation,

- \\(\sigma_\mathrm{lim}\in[0,\beta_1)\\), the limiting skill deviation,

- \\(w\in(0,\infty)\\), the contest weight.

Only \\(w\\) may vary between contests. Then, we set

\\[ (\beta_\mathrm{excess})^2 := \frac{(\beta_1)^2 - (\sigma_\mathrm{lim})^2}{w}, \\]

\\[ \beta^2 := (\beta_\mathrm{excess})^2 + (\sigma_\mathrm{lim})^2, \\]

\\[ \gamma := \frac{(\sigma_\mathrm{lim})^2}{\beta_\mathrm{excess}}. \\]

It can be shown that two consecutive contests, with weights \\(w_1,w_2\\), add the same cumulative amount of drift as one contest with weight \\(w_1+w_2\\). Furthermore, as a player acquires experience, we have in the limit that \\(\sigma\rightarrow\sigma_\mathrm{lim}\\).

TODO: consider eliminating \\(\beta_1\\) by fixing \\(\beta_1^2:=2(\sigma_\mathrm{lim})^2\\). Then in terms of the **relative weight**

\\[ w := \frac{(\sigma_\mathrm{lim})^2 + \gamma^2}{\beta^2}, \\]

the above formulas become

\\[ \beta^2 := \left(1 + \frac 1w\right)(\sigma_\mathrm{lim})^2, \\]

\\[ \gamma^2 := w (\sigma_\mathrm{lim})^2. \\]