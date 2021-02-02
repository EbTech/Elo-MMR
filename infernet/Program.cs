namespace infernet
{
    using System;
    using System.IO;
    using System.Linq;
    using System.Collections;
    using System.Collections.Generic;
    using Microsoft.ML.Probabilistic.Distributions;
    using Microsoft.ML.Probabilistic.Models;

    using Range = Microsoft.ML.Probabilistic.Models.Range;
    using Newtonsoft.Json;

    public class Contest
    {
        public int id;
        public string name;
        public long time_seconds;
        [Newtonsoft.Json.JsonConverter(typeof(TupleListConverter<string, int, int>))]
        public List<Tuple<string, int, int>> standings;
    }

    class Program
    {
        static void Main(string[] args)
        {
            // TrueSkill parameters
            const double mu_noob = 1500, sig_noob = 350, sig_perf = 175, sig_drift = 30, eps = 1;

            // Read in Codeforces files
            string CFPath = Path.Combine("..", "cache", "codeforces");

            var priorRatings = new Dictionary<string, Gaussian>();

            for (int contest_id = 0; contest_id <= 0; contest_id++)
            {
                string jsontext = System.IO.File.ReadAllText(Path.Combine(CFPath, $"{contest_id}.json"));
                Contest contest = JsonConvert.DeserializeObject<Contest>(jsontext);
                int N = contest.standings.Count;

                // Fill in missing priors
                for (int i = 0; i < N; i++)
                {
                    string playerName = contest.standings[i].Item1;
                    if (!priorRatings.ContainsKey(playerName))
                    {
                        priorRatings[playerName] = Gaussian.FromMeanAndVariance(mu_noob, sig_noob * sig_noob);
                    }
                }

                // Solve an instance of TrueSkill, using previous round ratings as priors
                // (Can also do a massive program with all participants together if retaining history)
                var playerSkills = Variable.Array<double>(new Range(N));
                for (int i = 0; i < N; i++)
                {
                    string playerName = contest.standings[i].Item1;
                    Gaussian prior = priorRatings[playerName];
                    double newVariance = prior.GetVariance() + sig_drift * sig_drift;
                    playerSkills[i] = Variable.GaussianFromMeanAndVariance(prior.GetMean(), newVariance);
                }

                for (int i = 1; i < N; i++)
                {
                    // The player performance is a noisy version of their skill
                    var winnerPerformance = Variable.GaussianFromMeanAndVariance(playerSkills[i - 1], sig_perf * sig_perf);
                    var loserPerformance = Variable.GaussianFromMeanAndVariance(playerSkills[i], sig_perf * sig_perf);
                    var perfDelta = winnerPerformance - loserPerformance;

                    if (contest.standings[i - 1].Item2 != contest.standings[i].Item2)
                    {
                        // The winner performed better in this game
                        Variable.ConstrainTrue(perfDelta > eps);
                    }
                    else
                    {
                        // The players tied
                        Variable.ConstrainBetween(perfDelta, -eps, eps);
                    }
                }

                // Run inference
                var inferenceEngine = new InferenceEngine();
                var inferredSkills = inferenceEngine.Infer<Gaussian[]>(playerSkills);

                // Save the posterior
                for (int i = 0; i < N; i++)
                {
                    string playerName = contest.standings[i].Item1;
                    priorRatings[playerName] = inferredSkills[i];
                }
            }

            // The inferred skills are uncertain, which is captured in their variance
            var orderedPlayerSkills = priorRatings
        .Select(kv => new { Player = kv.Key, Skill = kv.Value })
        .OrderByDescending(ps => ps.Skill.GetMean());

            foreach (var playerSkill in orderedPlayerSkills)
            {
                Console.WriteLine($"Player {playerSkill.Player}'s skill: {playerSkill.Skill}");
            }
            Console.WriteLine("Computations done");
        }
    }
}
