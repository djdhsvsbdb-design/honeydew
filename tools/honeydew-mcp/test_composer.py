import unittest

import composer


class ComposerTests(unittest.TestCase):
    def test_build_plan_is_repeatable_and_bounded(self):
        instruments = {"built_in": ["Drums", "Transistor Bass", "MiniSynth", "SuperSaw"]}
        effects = [{"name": name, "parameters": []} for name in
                   ("Reverb", "Delay", "Compressor", "Chorus")]
        first = composer.build_plan("uplifting trance, energetic", 8, 17, instruments, effects)
        second = composer.build_plan("uplifting trance, energetic", 8, 17, instruments, effects)
        self.assertEqual(first, second)
        self.assertEqual(first["brief"]["genre"], "trance")
        self.assertEqual(first["arrangement"]["tempo_bpm"], 138)
        self.assertEqual(len(first["arrangement"]["tracks"]), 4)
        for track in first["arrangement"]["tracks"]:
            self.assertEqual(track["bars"], 8)
            self.assertLessEqual(len(track["notes"]), 512)
            for note in track["notes"]:
                self.assertGreaterEqual(note["beat"], 0)
                self.assertLess(note["beat"] + note["length_beats"], 32.01)
                self.assertGreaterEqual(note["key"], 0)
                self.assertLessEqual(note["key"], 127)

    def test_non_four_four_meter_keeps_notes_inside_bars(self):
        plan = composer.build_plan(
            "house groove", 4, 5,
            {"built_in": ["Drums", "Transistor Bass", "MiniSynth", "SuperSaw"]},
            [{"name": "Reverb"}], beats_per_bar=3.0,
        )
        for track in plan["arrangement"]["tracks"]:
            for note in track["notes"]:
                self.assertLessEqual(note["beat"] + note["length_beats"], 12.0)

    def test_tempo_override_and_minimal_ambient(self):
        plan = composer.build_plan(
            "minimal ambient, dark", 2, 3,
            {"built_in": ["Drums", "MiniSynth"]},
            [{"name": "Reverb"}], 100,
        )
        self.assertEqual(plan["brief"]["tempo"], 100)
        self.assertEqual(plan["brief"]["mode"], "minor")
        self.assertEqual(plan["arrangement"]["tracks"][0]["notes"], [])
        self.assertEqual(len(plan["arrangement"]["tracks"][1]["notes"]), 2)


if __name__ == "__main__":
    unittest.main()
