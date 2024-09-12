import pyaici.server as aici


async def main(a, b, c, d):
    await aici.FixedTokens(a)
    await aici.label()
    await aici.FixedTokens(b)
    await aici.label()
    await aici.FixedTokens(c)
    await aici.label()
    await aici.FixedTokens(d)



aici.start(main(
    "a",
    "b",
    "c",
    "d",
))
