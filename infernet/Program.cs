namespace infernet
{
    using System;
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
        public int time_seconds;
        [Newtonsoft.Json.JsonConverter(typeof(TupleListConverter<string, int, int>))]
        public List<Tuple<string, int, int>> standings;
    }

    class Program
    {
        static void Main(string[] args)
        {
            // TrueSkill parameters
            double mu_noob = 1500, sig_noob = 200, perf_sig = 100;

            // Read in CF files
            string CFPath = "C:\\Users\\bb8\\Documents\\cs-projects\\EloR\\cache\\codeforces";

            var priorRatings = new Dictionary<string, Gaussian>();

            for (int contest_id = 1; contest_id <= 1; contest_id++)
            {
                string jsontext = System.IO.File.ReadAllText($"{CFPath}\\{contest_id}.json");
                Contest contest = JsonConvert.DeserializeObject<Contest>(jsontext);

                // Fill in missing priors
                for (int i = 0; i < contest.standings.Count; i++)
                {
                    var playerName = contest.standings[i].Item1;
                    if (!priorRatings.ContainsKey(playerName))
                    {
                        priorRatings[playerName] = Gaussian.FromMeanAndVariance(mu_noob, sig_noob);
                    }
                }

                // Number of participants in this contest
                var N = contest.standings.Count;

                // Solve an instance of TrueSkill, using previous round ratings as priors
                // (Can also do a massive program with all participants together if retaining history)
                var playerSkills = Variable.Array<double>(new Range(N));
                for (int i = 0; i < N; i++)
                {
                    var playerName = contest.standings[i].Item1;
                    Gaussian prior = priorRatings[playerName];
                    playerSkills[i] = Variable.GaussianFromMeanAndVariance(prior.GetMean(), prior.GetVariance());
                }

                for (int i = 0; i < N - 1; i++)
                {
                    // The player performance is a noisy version of their skill
                    var winnerPerformance = Variable.GaussianFromMeanAndVariance(playerSkills[i], perf_sig);
                    var loserPerformance = Variable.GaussianFromMeanAndVariance(playerSkills[i + 1], perf_sig);

                    // The winner performed better in this game
                    Variable.ConstrainTrue(winnerPerformance > loserPerformance);
                }

                // Run inference
                var inferenceEngine = new InferenceEngine();
                var inferredSkills = inferenceEngine.Infer<Gaussian[]>(playerSkills);

                // Save the posterior
                for (int i = 0; i < N; i++)
                {
                    var playerName = contest.standings[i].Item1;
                    priorRatings[playerName] = inferredSkills[i];
                }
            }

            // The inferred skills are uncertain, which is captured in their variance
            var orderedPlayerSkills = priorRatings
        .Select(kv => new { Player = kv.Key, Skill = kv.Value })
        .OrderByDescending(ps => ps.Skill.GetMean());

            foreach (var playerSkill in orderedPlayerSkills)
            {
                Console.WriteLine($"Player {playerSkill.Player} skill: {playerSkill.Skill}");
            }
            Console.WriteLine("Computations done");
        }
    }
}
