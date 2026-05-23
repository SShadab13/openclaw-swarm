#!/usr/bin/env python3
"""
ScrapeGraphAI Integration for OpenClaw Swarm
Smart web scraping with LLM-powered extraction.
"""

import json
import os
import sys
from typing import Dict, List, Optional
from dataclasses import dataclass
from datetime import datetime

try:
    from scrapegraphai.graphs import SmartScraperGraph, SearchGraph
    from scrapegraphai.models import OpenAI
    HAS_SCRAPEGRAPH = True
except ImportError:
    HAS_SCRAPEGRAPH = False
    print("⚠️ ScrapeGraphAI not installed. Install with: pip install scrapegraphai")


@dataclass
class ScrapedInsight:
    """Structured insight from web scraping."""
    url: str
    title: str
    content: str
    key_points: List[str]
    entities: List[str]
    confidence: float
    scraped_at: str
    source_type: str  # 'documentation', 'article', 'repo', 'paper'


class AIScraper:
    """AI-powered scraper for swarm knowledge acquisition."""
    
    def __init__(self, openai_key: Optional[str] = None):
        self.openai_key = openai_key or os.getenv('OPENAI_API_KEY')
        self.graph_config = None
        
        if HAS_SCRAPEGRAPH and self.openai_key:
            self.graph_config = {
                "llm": {
                    "api_key": self.openai_key,
                    "model": "openai/gpt-4o",
                },
                "verbose": True,
                "headless": False,
            }
    
    def scrape_url(self, url: str, prompt: str) -> Optional[ScrapedInsight]:
        """
        Scrape a URL with LLM-powered extraction.
        
        Example prompts:
        - "Extract all API endpoints and their parameters"
        - "Find code examples and explain what they do"
        - "Extract key concepts and definitions"
        - "Summarize the architecture described"
        """
        if not HAS_SCRAPEGRAPH:
            print("❌ ScrapeGraphAI not available")
            return None
        
        if not self.graph_config:
            print("❌ OpenAI API key not configured")
            return None
        
        try:
            # Create smart scraper graph
            scraper = SmartScraperGraph(
                prompt=prompt,
                source=url,
                config=self.graph_config
            )
            
            result = scraper.run()
            
            # Structure the result
            insight = ScrapedInsight(
                url=url,
                title=result.get('title', 'Unknown'),
                content=json.dumps(result, indent=2),
                key_points=result.get('key_points', []),
                entities=result.get('entities', []),
                confidence=result.get('confidence', 0.8),
                scraped_at=datetime.now().isoformat(),
                source_type=self._classify_source(url),
            )
            
            return insight
            
        except Exception as e:
            print(f"❌ Scraping failed: {e}")
            return None
    
    def search_and_scrape(self, query: str, max_results: int = 5) -> List[ScrapedInsight]:
        """
        Search the web and scrape results.
        Uses SearchGraph for multi-source scraping.
        """
        if not HAS_SCRAPEGRAPH:
            print("❌ ScrapeGraphAI not available")
            return []
        
        if not self.graph_config:
            print("❌ OpenAI API key not configured")
            return []
        
        try:
            search_graph = SearchGraph(
                prompt=query,
                config=self.graph_config
            )
            
            result = search_graph.run()
            
            insights = []
            for item in result.get('results', [])[:max_results]:
                insight = ScrapedInsight(
                    url=item.get('url', ''),
                    title=item.get('title', 'Unknown'),
                    content=item.get('content', ''),
                    key_points=item.get('key_points', []),
                    entities=item.get('entities', []),
                    confidence=item.get('confidence', 0.7),
                    scraped_at=datetime.now().isoformat(),
                    source_type='search_result',
                )
                insights.append(insight)
            
            return insights
            
        except Exception as e:
            print(f"❌ Search failed: {e}")
            return []
    
    def _classify_source(self, url: str) -> str:
        """Classify URL type for knowledge categorization."""
        if 'github.com' in url:
            return 'repo'
        elif 'arxiv.org' in url or 'doi.org' in url:
            return 'paper'
        elif 'docs.' in url or 'documentation' in url:
            return 'documentation'
        else:
            return 'article'
    
    def save_to_knowledge(self, insight: ScrapedInsight, db_path: str = "swarm_knowledge.db"):
        """Save scraped insight to knowledge base."""
        import sqlite3
        
        conn = sqlite3.connect(db_path)
        c = conn.cursor()
        
        # Ensure web_scrapes table exists
        c.execute('''
            CREATE TABLE IF NOT EXISTS web_scrapes (
                id INTEGER PRIMARY KEY,
                url TEXT UNIQUE,
                title TEXT,
                content TEXT,
                key_points TEXT,
                entities TEXT,
                confidence REAL,
                source_type TEXT,
                scraped_at TEXT
            )
        ''')
        
        c.execute('''
            INSERT OR REPLACE INTO web_scrapes
            (url, title, content, key_points, entities, confidence, source_type, scraped_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        ''', (
            insight.url,
            insight.title,
            insight.content,
            json.dumps(insight.key_points),
            json.dumps(insight.entities),
            insight.confidence,
            insight.source_type,
            insight.scraped_at,
        ))
        
        conn.commit()
        conn.close()
        
        print(f"💾 Saved scrape: {insight.title}")


def scrape_karpathy_repos():
    """Scrape Karpathy's key repositories for knowledge."""
    scraper = AIScraper()
    
    repos = [
        ("https://github.com/karpathy/nanoGPT", "Extract the architecture, training pipeline, and key design decisions. What makes this simple?"),
        ("https://github.com/karpathy/nanochat", "Extract the chat interface design, model architecture, and inference optimizations."),
        ("https://github.com/karpathy/llm.c", "Extract the C implementation details, CUDA optimizations, and educational approach."),
        ("https://github.com/karpathy/llama2.c", "Extract the inference engine design, quantization approach, and performance numbers."),
        ("https://github.com/karpathy/micrograd", "Extract the autograd engine design, neural net API, and educational value."),
    ]
    
    print("🤖 Scraping Karpathy repositories...")
    insights = []
    
    for url, prompt in repos:
        print(f"\n  🔍 {url}")
        insight = scraper.scrape_url(url, prompt)
        if insight:
            insights.append(insight)
            scraper.save_to_knowledge(insight)
    
    print(f"\n✅ Scraped {len(insights)} repositories")
    return insights


def main():
    """CLI for AI scraping."""
    import argparse
    
    parser = argparse.ArgumentParser(description='ScrapeGraphAI Integration')
    parser.add_argument('command', choices=['scrape', 'search', 'karpathy'])
    parser.add_argument('--url', help='URL to scrape')
    parser.add_argument('--prompt', help='Extraction prompt')
    parser.add_argument('--query', help='Search query')
    parser.add_argument('--db', default='swarm_knowledge.db', help='Knowledge DB path')
    
    args = parser.parse_args()
    
    scraper = AIScraper()
    
    if args.command == 'scrape':
        if not args.url or not args.prompt:
            print("Usage: python scrape.py scrape --url <url> --prompt <prompt>")
            sys.exit(1)
        
        insight = scraper.scrape_url(args.url, args.prompt)
        if insight:
            scraper.save_to_knowledge(insight, args.db)
            print(json.dumps(insight.__dict__, indent=2))
    
    elif args.command == 'search':
        if not args.query:
            print("Usage: python scrape.py search --query <query>")
            sys.exit(1)
        
        insights = scraper.search_and_scrape(args.query)
        for i in insights:
            scraper.save_to_knowledge(i, args.db)
        print(f"Found {len(insights)} results")
    
    elif args.command == 'karpathy':
        scrape_karpathy_repos()


if __name__ == '__main__':
    main()
