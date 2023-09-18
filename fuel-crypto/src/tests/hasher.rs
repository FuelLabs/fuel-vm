use crate::*;

#[test]
fn digest() {
    let input: [&'static [u8]; 14] = [
        b"I met a traveler from an antique land",
        b"Who said: 'Two vast and trunkless legs of stone'",
        b"Stand in the desert. Near them, on the sand,",
        b"Half sunk, a shattered visage lies, whose frown,",
        b"And wrinkled lip, and sneer of cold command,",
        b"Tell that its sculptor well those passions read",
        b"Which yet survive, stamped on these lifeless things,",
        b"The hand that mocked them and the heart that fed:",
        b"And on the pedestal these words appear:",
        b"'My name is Ozymandias, king of kings;'",
        b"Look on my works, ye Mighty, and despair!",
        b"Nothing beside remains. Round the decay",
        b"Of that colossal wreck, boundless and bare",
        b"The lone and level sands stretch far away.",
    ];

    let mut h = Hasher::default();

    input.iter().for_each(|i| h.input(i));

    let digest = h.finalize();

    let mut h = Hasher::default();

    h.extend(input.iter());

    let d = h.finalize();

    assert_eq!(digest, d);

    let d = input
        .iter()
        .fold(Hasher::default(), |h, i| h.chain(i))
        .finalize();

    assert_eq!(digest, d);

    let d = Hasher::default().extend_chain(input.iter()).finalize();

    assert_eq!(digest, d);

    let d = input.iter().collect::<Hasher>().finalize();

    assert_eq!(digest, d);
}
