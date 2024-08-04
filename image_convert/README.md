# image convert

一般的な静止画イメージをWebGL向けの画像フォーマット(DDS=DirectDraw Surface file format)に変換する。

imageクレートは非常に重くまた画像処理自体も重いので、可能なら画像変換をお粉はないほうが良い。
これが導入できなければ、pngを読むためにimageクレートを使う必要があり、ビルド時間が2秒が10秒まで伸びていた。

DDSの形式はいくつかあり、今回は一般デスクトップでは使えるDXT1を選んでいる。
https://developer.mozilla.org/en-US/docs/Web/API/WEBGL_compressed_texture_s3tc
