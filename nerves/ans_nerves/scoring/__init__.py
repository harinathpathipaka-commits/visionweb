"""Decision scoring engine — Advanced Multi-Factor ML scoring.

Scores every (action, tool, context) tuple across 6 dimensions:
1. Immediate outcome — Did the action work technically?
2. Goal advancement — Did it advance the goal?
3. Efficiency — How fast vs. expected? (sigmoid-scaled)
4. Consistency — How often does this action succeed in similar contexts?
5. Business impact — Long-term business value
6. Error penalty — Error severity via NLP matching (subtracted)

Feedback loop: action → outcome → score → embed → store → retrieve.
Stored in LanceDB with 384-dim context embeddings for ANN similarity search.
"""

from .embeddings import EmbeddingGenerator, get_embedding_generator
from .intelligence import ActionRecord, DecisionIntelligence, ScoredAction
from .scorer import (
    AdvancedMultiFactorScorer,
    DecisionScorer,
    FactorScores,
    ScoringWeights,
    classify_error_severity,
    sigmoid_efficiency,
)
from .store import LanceDBStore, get_store

__all__ = [
    "AdvancedMultiFactorScorer",
    "DecisionScorer",  # legacy alias for AdvancedMultiFactorScorer
    "DecisionIntelligence",
    "EmbeddingGenerator",
    "FactorScores",
    "LanceDBStore",
    "ScoringWeights",
    "ActionRecord",
    "ScoredAction",
    "classify_error_severity",
    "sigmoid_efficiency",
    "get_embedding_generator",
    "get_store",
]
