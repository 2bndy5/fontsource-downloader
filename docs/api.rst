API
===

.. currentmodule:: fontsource_downloader

.. automodule:: fontsource_downloader

    Basic API
    ---------

    .. autoclass:: FontSourceClient
        :members: download_font, css, css_self_hosted

    .. autoclass:: FontQuery
        :members:
        :special-members: __new__

    .. autoclass:: FontFileType
        :members:
        :special-members: __str__

        * ``FontFileType.Woff2`` = woff2
        * ``FontFileType.Woff`` = woff
        * ``FontFileType.Ttf`` = ttf

    .. autoclass:: Weight
        :members:
        :special-members: __int__

        * ``Weight.Thin`` = 100
        * ``Weight.ExtraLight`` = 200
        * ``Weight.Light`` = 300
        * ``Weight.Normal`` = 400
        * ``Weight.Medium`` = 500
        * ``Weight.SemiBold`` = 600
        * ``Weight.Bold`` = 700
        * ``Weight.ExtraBold`` = 800
        * ``Weight.Black`` = 900

    Exploring Cache
    ----------------

    .. automethod:: FontSourceClient.font_list_cache_info
    .. autoclass:: FontListCacheInfo
        :members:
    .. automethod:: FontSourceClient.family_cache_info
    .. autoclass:: FamilyCacheInfo
        :members:
    .. autoclass:: FontSourceFamily
        :members:
