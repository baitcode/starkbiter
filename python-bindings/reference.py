import starkbiter


class TraderAgent:

    def __init__(self, env: starkbiter.Env):
        self.account = env.create_account()
        self.token1 = "ETH"
        self.token2 = "STRK"

    def act(self, pool):
        # Example action: Buy 1 token0
        self.account.call(
            pool.swap(self.token1, self.token2, amount=1),
        )


class ArbitragerAgent:

    def __init__(self, env: starkbiter.Env):
        self.account = env.create_account()
        self.token1 = "ETH"
        self.token2 = "STRK"

    def act(self, pool, price: int):
        # Example action: Buy 1 token0
        self.account.call(
            pool.swap(self.token1, self.token2, amount=1),
        )


with starkbiter.get_environment("mainnet", block_id="0x0", block_hash="0x0") as env:
    simulate_blocks = 100

    pool = starkbiter.contracts.EkuboSwapper.deploy()

    trader = TraderAgent(env)
    arbitrager = ArbitragerAgent(env)

    # create sub

    blocks_produced = 0
    while blocks_produced < simulate_blocks:
        events = env.get_events("price_changed")  # poll subscription

        eth_price_changed = events.filter_latest(lambda e: e["token"] == "ETH")

        if eth_price_changed:
            trader.act(pool)
            arbitrager.act(pool, eth_price_changed.price)

        env.produce_new_block()
