#!/usr/bin/env python3
"""
Mycelix Mail - MATL Trust Score Bridge
Syncs trust scores from the 0TML MATL system to Holochain DHT
"""

import asyncio
import sys
import time
from pathlib import Path

# Add the 0TML directory to path for imports
sys.path.insert(0, str(Path(__file__).parent.parent.parent / "0TML" / "src"))

try:
    from holochain_client import HolochainClient
except ImportError:
    print("⚠️  holochain_client not installed. Install with: pip install holochain-client-python")
    sys.exit(1)


class MATLBridge:
    """
    Bridges MATL trust scores to Holochain

    Periodically queries the MATL system for trust scores and publishes
    them to the Holochain DHT where the trust_filter zome can access them.
    """

    def __init__(
        self,
        holochain_url: str = "ws://localhost:8888",
        matl_endpoint: str = "http://localhost:8080",
        sync_interval: int = 300  # 5 minutes
    ):
        self.holochain_url = holochain_url
        self.matl_endpoint = matl_endpoint
        self.sync_interval = sync_interval
        self.hc = None

    async def connect(self):
        """Connect to Holochain conductor"""
        print(f"🔌 Connecting to Holochain at {self.holochain_url}...")

        try:
            self.hc = await HolochainClient.connect(self.holochain_url)
            print("✅ Connected to Holochain")
        except Exception as e:
            print(f"❌ Failed to connect to Holochain: {e}")
            raise

    async def get_active_dids(self) -> list[str]:
        """
        Query Holochain for all DIDs that have sent or received messages

        In production, this would query the DHT for all unique sender DIDs.
        For MVP, we'll use a mock list.
        """
        # TODO: Implement real query
        # Should query all MailMessage entries and extract unique from_did values

        # Mock data for testing
        return [
            "did:mycelix:alice123",
            "did:mycelix:bob456",
            "did:mycelix:spammer789",
        ]

    async def get_matl_trust_score(self, did: str) -> dict:
        """
        Query MATL system for a DID's trust score

        In production, this would call the actual MATL API from 0TML.
        For MVP, we'll simulate scores.
        """
        # TODO: Connect to real MATL system
        # from zerotrustml.matl import MATLClient
        # matl = MATLClient(mode="mode1", oracle_endpoint=self.matl_endpoint)
        # score = await matl.get_composite_trust_score(did)

        # Simulated trust scores for testing
        if "spammer" in did:
            return {
                "did": did,
                "score": 0.05,  # Very low trust
                "pogq": 0.1,
                "tcdm": 0.0,
                "entropy": 0.05,
                "source": "matl_mode1"
            }
        elif "alice" in did:
            return {
                "did": did,
                "score": 0.85,  # High trust
                "pogq": 0.9,
                "tcdm": 0.8,
                "entropy": 0.85,
                "source": "matl_mode1"
            }
        else:
            return {
                "did": did,
                "score": 0.5,  # Neutral (new user)
                "pogq": 0.5,
                "tcdm": 0.5,
                "entropy": 0.5,
                "source": "matl_mode1"
            }

    async def update_holochain_trust_score(self, trust_data: dict):
        """
        Publish trust score to Holochain DHT via trust_filter zome
        """
        try:
            result = await self.hc.call_zome(
                cell_id=self.get_cell_id(),
                zome_name="trust_filter",
                fn_name="update_trust_score",
                payload={
                    "did": trust_data["did"],
                    "score": trust_data["score"],
                    "last_updated": int(time.time()),
                    "matl_source": trust_data["source"]
                }
            )
            print(f"  ✅ Updated {trust_data['did']}: {trust_data['score']:.2f}")
            return result
        except Exception as e:
            print(f"  ❌ Failed to update {trust_data['did']}: {e}")
            return None

    def get_cell_id(self):
        """
        Get the cell ID for the mycelix-mail DNA

        In production, this would be configured or discovered.
        For MVP, we need to get it from the conductor.
        """
        # TODO: Implement proper cell ID discovery
        # For now, return None and let the error guide us
        return None

    async def sync_trust_scores(self):
        """
        Main sync loop - periodically updates all trust scores
        """
        print(f"🔄 Starting trust score sync (interval: {self.sync_interval}s)")

        while True:
            try:
                print("\n📊 Syncing trust scores...")

                # Get all active DIDs
                dids = await self.get_active_dids()
                print(f"   Found {len(dids)} active DIDs")

                # Update each DID's trust score
                for did in dids:
                    # Get trust score from MATL
                    trust_data = await self.get_matl_trust_score(did)

                    # Publish to Holochain
                    await self.update_holochain_trust_score(trust_data)

                print(f"✅ Sync complete. Sleeping for {self.sync_interval}s...")

            except Exception as e:
                print(f"❌ Error during sync: {e}")

            await asyncio.sleep(self.sync_interval)

    async def run(self):
        """Start the bridge service"""
        await self.connect()
        await self.sync_trust_scores()


async def main():
    """Main entry point"""
    print("🍄 Mycelix Mail - MATL Trust Bridge")
    print("=" * 50)

    bridge = MATLBridge(
        holochain_url="ws://localhost:8888",
        matl_endpoint="http://localhost:8080",
        sync_interval=300  # 5 minutes
    )

    try:
        await bridge.run()
    except KeyboardInterrupt:
        print("\n\n👋 Shutting down gracefully...")
    except Exception as e:
        print(f"\n\n❌ Fatal error: {e}")
        sys.exit(1)


if __name__ == "__main__":
    asyncio.run(main())
