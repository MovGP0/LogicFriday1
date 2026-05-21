/* 00412923 FUN_00412923 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

undefined4 FUN_00412923(void)

{
  undefined4 uVar1;
  size_t sVar2;
  int iVar3;
  void *pvVar4;
  undefined4 *puVar5;
  HENHMETAFILE pHVar6;
  undefined4 extraout_ECX;
  int unaff_EBP;
  
  FUN_0043f30c();
  *(uint *)(unaff_EBP + -0x24) = DAT_00451a00 ^ *(uint *)(unaff_EBP + 4);
  *(undefined4 *)(unaff_EBP + -0x168) = extraout_ECX;
  *(undefined4 *)(unaff_EBP + -0x144) = *(undefined4 *)(*(int *)(unaff_EBP + 0xc) + 0x1668);
  *(undefined4 *)(unaff_EBP + -0x140) = *(undefined4 *)(*(int *)(unaff_EBP + 0xc) + 0x16d0);
  *(undefined4 *)(unaff_EBP + -0x14) = *(undefined4 *)(*(int *)(unaff_EBP + 0xc) + 0x16cc);
  *(undefined4 *)(unaff_EBP + -0x138) = *(undefined4 *)(*(int *)(unaff_EBP + 0xc) + 0x268);
  *(undefined4 *)(unaff_EBP + -0x134) = *(undefined4 *)(*(int *)(unaff_EBP + 0xc) + 0x26c);
  *(undefined4 *)(unaff_EBP + -0x130) = *(undefined4 *)(*(int *)(unaff_EBP + 0xc) + 0x274);
  *(undefined4 *)(unaff_EBP + -0x13c) = 0;
  uVar1 = FUN_0043e6f2(*(char **)(unaff_EBP + 0x10),"rb");
  *(undefined4 *)(unaff_EBP + -0x128) = uVar1;
  if (*(int *)(unaff_EBP + -0x128) == 0) {
    uVar1 = 0x2b0002;
  }
  else {
    sVar2 = _fread((void *)(unaff_EBP + -0x124),1,5,*(FILE **)(unaff_EBP + -0x128));
    *(size_t *)(unaff_EBP + -0x13c) = *(int *)(unaff_EBP + -0x13c) + sVar2;
    iVar3 = __stricmp((char *)(unaff_EBP + -0x124),"LTK1");
    if (iVar3 == 0) {
      sVar2 = _fread((void *)(unaff_EBP + -0x10),4,1,*(FILE **)(unaff_EBP + -0x128));
      *(size_t *)(unaff_EBP + -0x13c) = *(int *)(unaff_EBP + -0x13c) + sVar2;
      if (DAT_004519e4 < *(int *)(unaff_EBP + -0x10)) {
        _fclose(*(FILE **)(unaff_EBP + -0x128));
        uVar1 = 0x2c0000;
      }
      else {
        sVar2 = _fread((void *)(unaff_EBP + -0x150),4,1,*(FILE **)(unaff_EBP + -0x128));
        *(size_t *)(unaff_EBP + -0x13c) = *(int *)(unaff_EBP + -0x13c) + sVar2;
        sVar2 = _fread((void *)(unaff_EBP + -0x150),4,1,*(FILE **)(unaff_EBP + -0x128));
        *(size_t *)(unaff_EBP + -0x13c) = *(int *)(unaff_EBP + -0x13c) + sVar2;
        sVar2 = _fread((void *)(unaff_EBP + -0x1c),4,1,*(FILE **)(unaff_EBP + -0x128));
        *(size_t *)(unaff_EBP + -0x13c) = *(int *)(unaff_EBP + -0x13c) + sVar2;
        *(undefined4 *)(unaff_EBP + -300) = 0x2700;
        if (*(int *)(unaff_EBP + -0x1c) == 0x2700) {
          sVar2 = _fread(*(void **)(unaff_EBP + 0xc),*(size_t *)(unaff_EBP + -0x1c),1,
                         *(FILE **)(unaff_EBP + -0x128));
          *(size_t *)(unaff_EBP + -0x13c) = *(int *)(unaff_EBP + -0x13c) + sVar2;
          *(undefined4 *)(*(int *)(unaff_EBP + 0xc) + 600) = 1;
          *(undefined4 *)(*(int *)(unaff_EBP + 0xc) + 0x25c) = 0;
          *(undefined4 *)(*(int *)(unaff_EBP + 0xc) + 0x1668) = *(undefined4 *)(unaff_EBP + -0x144);
          *(undefined4 *)(*(int *)(unaff_EBP + 0xc) + 0x16d0) = *(undefined4 *)(unaff_EBP + -0x140);
          *(undefined4 *)(*(int *)(unaff_EBP + 0xc) + 0x16cc) = *(undefined4 *)(unaff_EBP + -0x14);
          *(undefined4 *)(*(int *)(unaff_EBP + 0xc) + 0x268) = *(undefined4 *)(unaff_EBP + -0x138);
          *(undefined4 *)(*(int *)(unaff_EBP + 0xc) + 0x26c) = *(undefined4 *)(unaff_EBP + -0x134);
          *(undefined4 *)(*(int *)(unaff_EBP + 0xc) + 0x274) = *(undefined4 *)(unaff_EBP + -0x130);
          *(undefined4 *)(*(int *)(unaff_EBP + 0xc) + 0x16c4) = 0;
          *(undefined4 *)(unaff_EBP + -0x1c) = **(undefined4 **)(unaff_EBP + 0xc);
          *(undefined4 *)(unaff_EBP + -0x18) = 0;
          while (*(uint *)(unaff_EBP + -0x18) < *(uint *)(*(int *)(unaff_EBP + 0xc) + 200)) {
            pvVar4 = _malloc(*(int *)(unaff_EBP + -0x1c) << 2);
            *(void **)(*(int *)(unaff_EBP + 0xc) + 0x84 + *(int *)(unaff_EBP + -0x18) * 4) = pvVar4;
            if (*(int *)(*(int *)(unaff_EBP + 0xc) + 0x84 + *(int *)(unaff_EBP + -0x18) * 4) == 0) {
              _fclose(*(FILE **)(unaff_EBP + -0x128));
              uVar1 = 0x4000f;
              goto LAB_00413372;
            }
            sVar2 = _fread(*(void **)(*(int *)(unaff_EBP + 0xc) + 0x84 +
                                     *(int *)(unaff_EBP + -0x18) * 4),4,
                           *(size_t *)(unaff_EBP + -0x1c),*(FILE **)(unaff_EBP + -0x128));
            *(size_t *)(unaff_EBP + -0x13c) = *(int *)(unaff_EBP + -0x13c) + sVar2;
            *(int *)(unaff_EBP + -0x18) = *(int *)(unaff_EBP + -0x18) + 1;
          }
          if (*(int *)(*(int *)(unaff_EBP + 0xc) + 0x23c) != 0) {
            *(undefined4 *)(unaff_EBP + -0x1c) = *(undefined4 *)(*(int *)(unaff_EBP + 0xc) + 500);
            pvVar4 = _malloc(*(int *)(unaff_EBP + -0x1c) * 0xc);
            *(void **)(*(int *)(unaff_EBP + 0xc) + 0x1f8) = pvVar4;
            sVar2 = _fread(*(void **)(*(int *)(unaff_EBP + 0xc) + 0x1f8),0xc,
                           *(size_t *)(unaff_EBP + -0x1c),*(FILE **)(unaff_EBP + -0x128));
            *(size_t *)(unaff_EBP + -0x13c) = *(int *)(unaff_EBP + -0x13c) + sVar2;
            *(undefined4 *)(unaff_EBP + -0x18) = 0;
            while (*(uint *)(unaff_EBP + -0x18) < *(uint *)(*(int *)(unaff_EBP + 0xc) + 200)) {
              pvVar4 = _malloc(*(int *)(unaff_EBP + -0x1c) << 2);
              *(void **)(*(int *)(unaff_EBP + 0xc) + 0x1fc + *(int *)(unaff_EBP + -0x18) * 4) =
                   pvVar4;
              sVar2 = _fread(*(void **)(*(int *)(unaff_EBP + 0xc) + 0x1fc +
                                       *(int *)(unaff_EBP + -0x18) * 4),4,
                             *(size_t *)(unaff_EBP + -0x1c),*(FILE **)(unaff_EBP + -0x128));
              *(size_t *)(unaff_EBP + -0x13c) = *(int *)(unaff_EBP + -0x13c) + sVar2;
              *(int *)(unaff_EBP + -0x18) = *(int *)(unaff_EBP + -0x18) + 1;
            }
          }
          if (1 < *(uint *)(*(int *)(unaff_EBP + 0xc) + 0x165c)) {
            pvVar4 = _realloc(*(void **)(*(int *)(unaff_EBP + 0xc) + 0x268),
                              *(int *)(*(int *)(unaff_EBP + 0xc) + 0x165c) * 0x7fff);
            *(void **)(*(int *)(unaff_EBP + 0xc) + 0x268) = pvVar4;
          }
          **(undefined1 **)(*(int *)(unaff_EBP + 0xc) + 0x268) = 0;
          sVar2 = _fread((void *)(unaff_EBP + -0x1c),4,1,*(FILE **)(unaff_EBP + -0x128));
          *(size_t *)(unaff_EBP + -0x13c) = *(int *)(unaff_EBP + -0x13c) + sVar2;
          if (*(int *)(unaff_EBP + -0x1c) != 0) {
            sVar2 = _fread(*(void **)(*(int *)(unaff_EBP + 0xc) + 0x268),1,
                           *(size_t *)(unaff_EBP + -0x1c),*(FILE **)(unaff_EBP + -0x128));
            *(size_t *)(unaff_EBP + -0x13c) = *(int *)(unaff_EBP + -0x13c) + sVar2;
          }
          if (1 < *(uint *)(*(int *)(unaff_EBP + 0xc) + 0x1660)) {
            pvVar4 = _realloc(*(void **)(*(int *)(unaff_EBP + 0xc) + 0x26c),
                              *(int *)(*(int *)(unaff_EBP + 0xc) + 0x1660) * 0x7fff);
            *(void **)(*(int *)(unaff_EBP + 0xc) + 0x26c) = pvVar4;
          }
          **(undefined1 **)(*(int *)(unaff_EBP + 0xc) + 0x26c) = 0;
          sVar2 = _fread((void *)(unaff_EBP + -0x1c),4,1,*(FILE **)(unaff_EBP + -0x128));
          *(size_t *)(unaff_EBP + -0x13c) = *(int *)(unaff_EBP + -0x13c) + sVar2;
          if (*(int *)(unaff_EBP + -0x1c) != 0) {
            sVar2 = _fread(*(void **)(*(int *)(unaff_EBP + 0xc) + 0x26c),1,
                           *(size_t *)(unaff_EBP + -0x1c),*(FILE **)(unaff_EBP + -0x128));
            *(size_t *)(unaff_EBP + -0x13c) = *(int *)(unaff_EBP + -0x13c) + sVar2;
          }
          sVar2 = _fread((void *)(unaff_EBP + -0x1c),4,1,*(FILE **)(unaff_EBP + -0x128));
          *(size_t *)(unaff_EBP + -0x13c) = *(int *)(unaff_EBP + -0x13c) + sVar2;
          if (*(int *)(unaff_EBP + -0x1c) != 0) {
            pvVar4 = _malloc(*(int *)(unaff_EBP + -0x1c) + 4);
            *(void **)(*(int *)(unaff_EBP + 0xc) + 0x270) = pvVar4;
            sVar2 = _fread(*(void **)(*(int *)(unaff_EBP + 0xc) + 0x270),1,
                           *(size_t *)(unaff_EBP + -0x1c),*(FILE **)(unaff_EBP + -0x128));
            *(size_t *)(unaff_EBP + -0x13c) = *(int *)(unaff_EBP + -0x13c) + sVar2;
          }
          if (1 < *(uint *)(*(int *)(unaff_EBP + 0xc) + 0x1664)) {
            pvVar4 = _realloc(*(void **)(*(int *)(unaff_EBP + 0xc) + 0x274),
                              *(int *)(*(int *)(unaff_EBP + 0xc) + 0x1664) * 0x7fff);
            *(void **)(*(int *)(unaff_EBP + 0xc) + 0x274) = pvVar4;
          }
          **(undefined1 **)(*(int *)(unaff_EBP + 0xc) + 0x274) = 0;
          sVar2 = _fread((void *)(unaff_EBP + -0x1c),4,1,*(FILE **)(unaff_EBP + -0x128));
          *(size_t *)(unaff_EBP + -0x13c) = *(int *)(unaff_EBP + -0x13c) + sVar2;
          if (*(int *)(unaff_EBP + -0x1c) != 0) {
            sVar2 = _fread(*(void **)(*(int *)(unaff_EBP + 0xc) + 0x274),1,
                           *(size_t *)(unaff_EBP + -0x1c),*(FILE **)(unaff_EBP + -0x128));
            *(size_t *)(unaff_EBP + -0x13c) = *(int *)(unaff_EBP + -0x13c) + sVar2;
          }
          *(undefined4 *)(unaff_EBP + -0x1c) = *(undefined4 *)(*(int *)(unaff_EBP + 0xc) + 0x1650);
          if (*(int *)(unaff_EBP + -0x1c) != 0) {
            *(undefined4 *)(unaff_EBP + -0x154) =
                 *(undefined4 *)(*(int *)(unaff_EBP + 0xc) + 0x1650);
            pvVar4 = operator_new(*(int *)(unaff_EBP + -0x154) * 0xfc + 4);
            *(void **)(unaff_EBP + -0x15c) = pvVar4;
            *(undefined4 *)(unaff_EBP + -4) = 0;
            if (*(int *)(unaff_EBP + -0x15c) == 0) {
              *(undefined4 *)(unaff_EBP + -0x16c) = 0;
            }
            else {
              **(undefined4 **)(unaff_EBP + -0x15c) = *(undefined4 *)(unaff_EBP + -0x154);
              _eh_vector_constructor_iterator_
                        ((void *)(*(int *)(unaff_EBP + -0x15c) + 4),0xfc,
                         *(int *)(unaff_EBP + -0x154),FUN_004175df,FUN_0043961a);
              *(int *)(unaff_EBP + -0x16c) = *(int *)(unaff_EBP + -0x15c) + 4;
            }
            *(undefined4 *)(unaff_EBP + -0x158) = *(undefined4 *)(unaff_EBP + -0x16c);
            *(undefined4 *)(unaff_EBP + -4) = 0xffffffff;
            *(undefined4 *)(*(int *)(unaff_EBP + 0xc) + 0x3a4) = *(undefined4 *)(unaff_EBP + -0x158)
            ;
            sVar2 = _fread(*(void **)(*(int *)(unaff_EBP + 0xc) + 0x3a4),0xfc,
                           *(size_t *)(unaff_EBP + -0x1c),*(FILE **)(unaff_EBP + -0x128));
            *(size_t *)(unaff_EBP + -0x13c) = *(int *)(unaff_EBP + -0x13c) + sVar2;
          }
          *(undefined4 *)(unaff_EBP + -0x1c) = *(undefined4 *)(*(int *)(unaff_EBP + 0xc) + 0x16c8);
          if (*(int *)(unaff_EBP + -0x1c) != 0) {
            if (10000 < *(uint *)(*(int *)(unaff_EBP + 0xc) + 0x16c0)) {
              pvVar4 = _realloc(*(void **)(*(int *)(unaff_EBP + 0xc) + 0x16d0),
                                *(int *)(*(int *)(unaff_EBP + 0xc) + 0x16c0) << 2);
              *(void **)(*(int *)(unaff_EBP + 0xc) + 0x16d0) = pvVar4;
            }
            *(undefined4 *)(unaff_EBP + -0x18) = 0;
            while (*(uint *)(unaff_EBP + -0x18) < *(uint *)(*(int *)(unaff_EBP + 0xc) + 0x16c8)) {
              pvVar4 = operator_new(0x50);
              *(void **)(unaff_EBP + -0x164) = pvVar4;
              *(undefined4 *)(unaff_EBP + -4) = 1;
              if (*(int *)(unaff_EBP + -0x164) == 0) {
                *(undefined4 *)(unaff_EBP + -0x170) = 0;
              }
              else {
                puVar5 = FUN_0043ab51(*(undefined4 **)(unaff_EBP + -0x164));
                *(undefined4 **)(unaff_EBP + -0x170) = puVar5;
              }
              *(undefined4 *)(unaff_EBP + -0x160) = *(undefined4 *)(unaff_EBP + -0x170);
              *(undefined4 *)(unaff_EBP + -4) = 0xffffffff;
              *(undefined4 *)
               (*(int *)(*(int *)(unaff_EBP + 0xc) + 0x16d0) + *(int *)(unaff_EBP + -0x18) * 4) =
                   *(undefined4 *)(unaff_EBP + -0x160);
              sVar2 = _fread(*(void **)(*(int *)(*(int *)(unaff_EBP + 0xc) + 0x16d0) +
                                       *(int *)(unaff_EBP + -0x18) * 4),0x50,1,
                             *(FILE **)(unaff_EBP + -0x128));
              *(size_t *)(unaff_EBP + -0x13c) = *(int *)(unaff_EBP + -0x13c) + sVar2;
              pvVar4 = _malloc(*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + 0xc) + 0x16d0) +
                                                *(int *)(unaff_EBP + -0x18) * 4) + 0x28) * 0x14);
              *(void **)(*(int *)(*(int *)(*(int *)(unaff_EBP + 0xc) + 0x16d0) +
                                 *(int *)(unaff_EBP + -0x18) * 4) + 0x2c) = pvVar4;
              *(undefined4 *)(unaff_EBP + -0x150) = 0;
              while (*(int *)(unaff_EBP + -0x150) <
                     *(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + 0xc) + 0x16d0) +
                                      *(int *)(unaff_EBP + -0x18) * 4) + 0x28)) {
                sVar2 = _fread((void *)(*(int *)(unaff_EBP + -0x150) * 0x14 +
                                       *(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + 0xc) + 0x16d0
                                                                 ) + *(int *)(unaff_EBP + -0x18) * 4
                                                        ) + 0x2c)),0x14,1,
                               *(FILE **)(unaff_EBP + -0x128));
                *(size_t *)(unaff_EBP + -0x13c) = *(int *)(unaff_EBP + -0x13c) + sVar2;
                *(int *)(unaff_EBP + -0x150) = *(int *)(unaff_EBP + -0x150) + 1;
              }
              pvVar4 = _malloc(*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + 0xc) + 0x16d0) +
                                                *(int *)(unaff_EBP + -0x18) * 4) + 0x30) * 0x14);
              *(void **)(*(int *)(*(int *)(*(int *)(unaff_EBP + 0xc) + 0x16d0) +
                                 *(int *)(unaff_EBP + -0x18) * 4) + 0x34) = pvVar4;
              *(undefined4 *)(unaff_EBP + -0x150) = 0;
              while (*(int *)(unaff_EBP + -0x150) <
                     *(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + 0xc) + 0x16d0) +
                                      *(int *)(unaff_EBP + -0x18) * 4) + 0x30)) {
                sVar2 = _fread((void *)(*(int *)(unaff_EBP + -0x150) * 0x14 +
                                       *(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + 0xc) + 0x16d0
                                                                 ) + *(int *)(unaff_EBP + -0x18) * 4
                                                        ) + 0x34)),0x14,1,
                               *(FILE **)(unaff_EBP + -0x128));
                *(size_t *)(unaff_EBP + -0x13c) = *(int *)(unaff_EBP + -0x13c) + sVar2;
                *(int *)(unaff_EBP + -0x150) = *(int *)(unaff_EBP + -0x150) + 1;
              }
              *(int *)(unaff_EBP + -0x18) = *(int *)(unaff_EBP + -0x18) + 1;
            }
            sVar2 = _fread((void *)(unaff_EBP + -0x20),4,1,*(FILE **)(unaff_EBP + -0x128));
            *(size_t *)(unaff_EBP + -0x13c) = *(int *)(unaff_EBP + -0x13c) + sVar2;
            if (*(int *)(unaff_EBP + -0x20) != 0) {
              pvVar4 = _malloc(*(size_t *)(unaff_EBP + -0x20));
              *(void **)(unaff_EBP + -0x148) = pvVar4;
              sVar2 = _fread(*(void **)(unaff_EBP + -0x148),1,*(size_t *)(unaff_EBP + -0x20),
                             *(FILE **)(unaff_EBP + -0x128));
              *(size_t *)(unaff_EBP + -0x13c) = *(int *)(unaff_EBP + -0x13c) + sVar2;
              pHVar6 = SetEnhMetaFileBits(*(UINT *)(unaff_EBP + -0x20),
                                          *(BYTE **)(unaff_EBP + -0x148));
              *(HENHMETAFILE *)(*(int *)(unaff_EBP + 0xc) + 0x16b0) = pHVar6;
              *(undefined4 *)(*(int *)(unaff_EBP + 0xc) + 0x1688) =
                   *(undefined4 *)(*(int *)(unaff_EBP + 0xc) + 0x16b0);
              _free(*(void **)(unaff_EBP + -0x148));
              *(undefined4 *)(unaff_EBP + -0x148) = 0;
            }
          }
          sVar2 = _fread(&DAT_00452ea8,4,1,*(FILE **)(unaff_EBP + -0x128));
          *(size_t *)(unaff_EBP + -0x13c) = *(int *)(unaff_EBP + -0x13c) + sVar2;
          sVar2 = _fread(&DAT_00452ea0,4,1,*(FILE **)(unaff_EBP + -0x128));
          *(size_t *)(unaff_EBP + -0x13c) = *(int *)(unaff_EBP + -0x13c) + sVar2;
          sVar2 = _fread(&DAT_00452e9c,4,1,*(FILE **)(unaff_EBP + -0x128));
          *(size_t *)(unaff_EBP + -0x13c) = *(int *)(unaff_EBP + -0x13c) + sVar2;
          sVar2 = _fread((void *)(unaff_EBP + -0x14c),4,1,*(FILE **)(unaff_EBP + -0x128));
          *(size_t *)(unaff_EBP + -0x13c) = *(int *)(unaff_EBP + -0x13c) + sVar2;
          _fclose(*(FILE **)(unaff_EBP + -0x128));
          uVar1 = 0;
        }
        else {
          _fclose(*(FILE **)(unaff_EBP + -0x128));
          uVar1 = 0x250000;
        }
      }
    }
    else {
      _fclose(*(FILE **)(unaff_EBP + -0x128));
      uVar1 = 0x250000;
    }
  }
LAB_00413372:
  ExceptionList = *(void **)(unaff_EBP + -0xc);
  return uVar1;
}
