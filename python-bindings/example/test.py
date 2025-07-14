import json
import asyncio
import python_bindings

from nethermind.starknet_abi.core import StarknetAbi


MAINNET = "0x534e5f4d41494e"

fork_params = python_bindings.ForkParams(
    "https://starknet-mainnet.public.blastapi.io", 1521205, "0x7aabf76192d3d16fe8bda54c0e7d0a9843c21fe20dd23704366bad38d57dc30"
)


abi = StarknetAbi.from_json(json.loads(
    python_bindings.contracts.ERC20_CONTRACT_SIERRA
)["abi"])

ETH_ERC20_MAINNET = "0x049d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7"
EKUBO_CORE_MAINNET = "0x00000005dd3d2f4429af886cd1a3b08289dbcea99a294197e9eb43b0e0325b4b"

balance_of_function = abi.functions["balanceOf"]

balance_of_calldata = balance_of_function.encode({
    "account": EKUBO_CORE_MAINNET
})

mint_function = abi.functions["mint"]


mint_calldata = mint_function.encode({
    "recipient": EKUBO_CORE_MAINNET,
    "amount": 1000000000000000000,
})

python_bindings.set_tracing()


async def main():
    print("Starting Starknet Python Bindings Example...", fork_params)

    env_label = await python_bindings.create_environment("test_env", MAINNET, fork_params)
    print("Environment created:", env_label)

    middleware_id = await python_bindings.create_middleware(env_label)
    print("Middleware created:", middleware_id)

    await python_bindings.set_gas_price(
        middleware_id,
        gas_price_wei=1,
        gas_price_fri=1,
        data_gas_price_wei=1,
        data_gas_price_fri=1,
        l2_gas_price_wei=1,
        l2_gas_price_fri=1,
        generate_block=True,
    )

    account_id = await python_bindings.create_account(middleware_id, "0x36078334509b514626504edc9fb252328d1a240e4e948bef8d0c08dff45927f")
    print("Account created:", account_id)

    call = python_bindings.Call(
        to=ETH_ERC20_MAINNET,
        selector=balance_of_function.signature.hex(),
        calldata=balance_of_calldata,
    )
    res = await python_bindings.call(middleware_id, call, python_bindings.BlockId.from_tag("latest"))
    print("Call result:", res)

    # await python_bindings.account_execute(account_id, [
    #     python_bindings.Call(
    #         to=ETH_ERC20_MAINNET,
    #         selector=mint_function.signature.hex(),
    #         calldata=mint_calldata,
    #     )
    # ])

    subscripiton_id = await python_bindings.create_subscription(middleware_id)

    while True:
        await python_bindings.do_swap()

        await python_bindings.create_block()
        events = await python_bindings.poll_subscription(subscription_id)


with asyncio.Runner() as runner:
    runner.run(main())
