/* 00427c43 FUN_00427c43 */

undefined4 FUN_00427c43(void)

{
  undefined4 uVar1;
  void *pvVar2;
  undefined4 *puVar3;
  undefined4 extraout_ECX;
  int iVar4;
  int iVar5;
  int iVar6;
  int unaff_EBP;
  undefined4 *puVar7;
  
  FUN_0043f30c();
  *(undefined4 *)(unaff_EBP + -0x7c) = extraout_ECX;
  *(undefined4 *)(unaff_EBP + -0x3c) = 0;
  *(undefined4 *)(unaff_EBP + -0x34) = 0;
  *(undefined4 *)(unaff_EBP + -0x10) = *(undefined4 *)(*(int *)(unaff_EBP + -0x7c) + 0x1654);
  while (*(int *)(unaff_EBP + -0x10) < *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x1650)) {
    if ((*(int *)(*(int *)(*(int *)(unaff_EBP + -0x7c) + 0x3a4) + 0x48 +
                 *(int *)(unaff_EBP + -0x10) * 0xfc) == 0) &&
       (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x7c) + 0x3a4) + 0xbc +
                *(int *)(unaff_EBP + -0x10) * 0xfc) != 0)) {
      *(int *)(unaff_EBP + -0x3c) = *(int *)(unaff_EBP + -0x3c) + 1;
    }
    *(int *)(unaff_EBP + -0x10) = *(int *)(unaff_EBP + -0x10) + 1;
  }
  if (*(int *)(unaff_EBP + -0x3c) == 0) {
    uVar1 = 0;
  }
  else {
    *(undefined4 *)(unaff_EBP + -0x44) = *(undefined4 *)(*(int *)(unaff_EBP + -0x7c) + 0x1650);
    pvVar2 = operator_new(*(int *)(unaff_EBP + -0x44) * 0xfc + 4);
    *(void **)(unaff_EBP + -0x4c) = pvVar2;
    *(undefined4 *)(unaff_EBP + -4) = 0;
    if (*(int *)(unaff_EBP + -0x4c) == 0) {
      *(undefined4 *)(unaff_EBP + -0x80) = 0;
    }
    else {
      **(undefined4 **)(unaff_EBP + -0x4c) = *(undefined4 *)(unaff_EBP + -0x44);
      _eh_vector_constructor_iterator_
                ((void *)(*(int *)(unaff_EBP + -0x4c) + 4),0xfc,*(int *)(unaff_EBP + -0x44),
                 FUN_004175df,FUN_0043961a);
      *(int *)(unaff_EBP + -0x80) = *(int *)(unaff_EBP + -0x4c) + 4;
    }
    *(undefined4 *)(unaff_EBP + -0x48) = *(undefined4 *)(unaff_EBP + -0x80);
    *(undefined4 *)(unaff_EBP + -4) = 0xffffffff;
    *(undefined4 *)(unaff_EBP + -0x30) = *(undefined4 *)(unaff_EBP + -0x48);
    *(undefined4 *)(unaff_EBP + -0x10) = 0;
    while (*(int *)(unaff_EBP + -0x10) < *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x1650)) {
      puVar3 = (undefined4 *)
               (*(int *)(*(int *)(unaff_EBP + -0x7c) + 0x3a4) + *(int *)(unaff_EBP + -0x10) * 0xfc);
      puVar7 = (undefined4 *)(*(int *)(unaff_EBP + -0x30) + *(int *)(unaff_EBP + -0x10) * 0xfc);
      for (iVar4 = 0x3f; iVar4 != 0; iVar4 = iVar4 + -1) {
        *puVar7 = *puVar3;
        puVar3 = puVar3 + 1;
        puVar7 = puVar7 + 1;
      }
      *(int *)(unaff_EBP + -0x10) = *(int *)(unaff_EBP + -0x10) + 1;
    }
    *(undefined4 *)(unaff_EBP + -0x54) = *(undefined4 *)(*(int *)(unaff_EBP + -0x7c) + 0x3a4);
    *(undefined4 *)(unaff_EBP + -0x50) = *(undefined4 *)(unaff_EBP + -0x54);
    if (*(int *)(unaff_EBP + -0x50) == 0) {
      *(undefined4 *)(unaff_EBP + -0x84) = 0;
    }
    else {
      pvVar2 = FUN_0041338b(*(void **)(unaff_EBP + -0x50),3);
      *(void **)(unaff_EBP + -0x84) = pvVar2;
    }
    *(int *)(unaff_EBP + -0x58) =
         *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x1650) + *(int *)(unaff_EBP + -0x3c);
    pvVar2 = operator_new(*(int *)(unaff_EBP + -0x58) * 0xfc + 4);
    *(void **)(unaff_EBP + -0x60) = pvVar2;
    *(undefined4 *)(unaff_EBP + -4) = 1;
    if (*(int *)(unaff_EBP + -0x60) == 0) {
      *(undefined4 *)(unaff_EBP + -0x88) = 0;
    }
    else {
      **(undefined4 **)(unaff_EBP + -0x60) = *(undefined4 *)(unaff_EBP + -0x58);
      _eh_vector_constructor_iterator_
                ((void *)(*(int *)(unaff_EBP + -0x60) + 4),0xfc,*(int *)(unaff_EBP + -0x58),
                 FUN_004175df,FUN_0043961a);
      *(int *)(unaff_EBP + -0x88) = *(int *)(unaff_EBP + -0x60) + 4;
    }
    *(undefined4 *)(unaff_EBP + -0x5c) = *(undefined4 *)(unaff_EBP + -0x88);
    *(undefined4 *)(unaff_EBP + -4) = 0xffffffff;
    *(undefined4 *)(*(int *)(unaff_EBP + -0x7c) + 0x3a4) = *(undefined4 *)(unaff_EBP + -0x5c);
    *(undefined4 *)(unaff_EBP + -0x10) = 0;
    while (*(int *)(unaff_EBP + -0x10) < *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x1658)) {
      puVar3 = (undefined4 *)(*(int *)(unaff_EBP + -0x30) + *(int *)(unaff_EBP + -0x10) * 0xfc);
      puVar7 = (undefined4 *)
               (*(int *)(*(int *)(unaff_EBP + -0x7c) + 0x3a4) + *(int *)(unaff_EBP + -0x10) * 0xfc);
      for (iVar4 = 0x3f; iVar4 != 0; iVar4 = iVar4 + -1) {
        *puVar7 = *puVar3;
        puVar3 = puVar3 + 1;
        puVar7 = puVar7 + 1;
      }
      *(int *)(unaff_EBP + -0x10) = *(int *)(unaff_EBP + -0x10) + 1;
    }
    *(undefined4 *)(unaff_EBP + -0x10) = *(undefined4 *)(*(int *)(unaff_EBP + -0x7c) + 0x1658);
    while (*(int *)(unaff_EBP + -0x10) < *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x1650)) {
      puVar3 = (undefined4 *)(*(int *)(unaff_EBP + -0x30) + *(int *)(unaff_EBP + -0x10) * 0xfc);
      puVar7 = (undefined4 *)
               (*(int *)(*(int *)(unaff_EBP + -0x7c) + 0x3a4) +
               (*(int *)(unaff_EBP + -0x10) + *(int *)(unaff_EBP + -0x3c)) * 0xfc);
      for (iVar4 = 0x3f; iVar4 != 0; iVar4 = iVar4 + -1) {
        *puVar7 = *puVar3;
        puVar3 = puVar3 + 1;
        puVar7 = puVar7 + 1;
      }
      *(int *)(unaff_EBP + -0x10) = *(int *)(unaff_EBP + -0x10) + 1;
    }
    *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x1650) =
         *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x1650) + *(int *)(unaff_EBP + -0x3c);
    *(undefined4 *)(unaff_EBP + -0x1c) = *(undefined4 *)(*(int *)(unaff_EBP + -0x7c) + 0x1658);
    *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x1658) =
         *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x1658) + *(int *)(unaff_EBP + -0x3c);
    *(undefined4 *)(unaff_EBP + -0x68) = *(undefined4 *)(unaff_EBP + -0x30);
    *(undefined4 *)(unaff_EBP + -100) = *(undefined4 *)(unaff_EBP + -0x68);
    if (*(int *)(unaff_EBP + -100) == 0) {
      *(undefined4 *)(unaff_EBP + -0x8c) = 0;
    }
    else {
      pvVar2 = FUN_0041338b(*(void **)(unaff_EBP + -100),3);
      *(void **)(unaff_EBP + -0x8c) = pvVar2;
    }
    *(undefined4 *)(unaff_EBP + -0x10) = *(undefined4 *)(*(int *)(unaff_EBP + -0x7c) + 0x1658);
    while (*(int *)(unaff_EBP + -0x10) < *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x1650)) {
      if ((*(int *)(*(int *)(*(int *)(unaff_EBP + -0x7c) + 0x3a4) + 0x1c +
                   *(int *)(unaff_EBP + -0x10) * 0xfc) != -2) &&
         (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x7c) + 0x3a4) + 0x1c +
                  *(int *)(unaff_EBP + -0x10) * 0xfc) != -1)) {
        *(undefined4 *)(unaff_EBP + -0x14) =
             *(undefined4 *)
              (*(int *)(*(int *)(unaff_EBP + -0x7c) + 0x3a4) + 0xe4 +
              *(int *)(unaff_EBP + -0x10) * 0xfc);
        *(undefined4 *)
         (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16d0) + *(int *)(unaff_EBP + -0x14) * 4)
         + 4) = *(undefined4 *)(unaff_EBP + -0x10);
      }
      *(int *)(unaff_EBP + -0x10) = *(int *)(unaff_EBP + -0x10) + 1;
    }
    *(undefined4 *)(unaff_EBP + -0x38) = *(undefined4 *)(unaff_EBP + -0x1c);
    *(undefined4 *)(unaff_EBP + -0x10) = *(undefined4 *)(*(int *)(unaff_EBP + -0x7c) + 0x1654);
    while (*(int *)(unaff_EBP + -0x10) < *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x1650)) {
      if (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x7c) + 0x3a4) + 0x48 +
                  *(int *)(unaff_EBP + -0x10) * 0xfc) == 0) {
        *(undefined4 *)(unaff_EBP + -0x34) = 0;
        *(undefined4 *)(unaff_EBP + -0x40) = 0;
        while (*(int *)(unaff_EBP + -0x40) <
               *(int *)(*(int *)(*(int *)(unaff_EBP + -0x7c) + 0x3a4) + 0x18 +
                       *(int *)(unaff_EBP + -0x10) * 0xfc)) {
          if ((*(int *)(*(int *)(unaff_EBP + -0x10) * 0xfc +
                        *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x3a4) + 0x1c +
                       *(int *)(unaff_EBP + -0x40) * 4) == -2) ||
             (*(int *)(*(int *)(unaff_EBP + -0x10) * 0xfc +
                       *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x3a4) + 0x1c +
                      *(int *)(unaff_EBP + -0x40) * 4) == -1)) {
            if ((*(int *)(unaff_EBP + -0x34) == 0) ||
               ((((*(int *)(*(int *)(unaff_EBP + -0x10) * 0xfc +
                           *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x3a4)) != 1 &&
                  (*(int *)(*(int *)(unaff_EBP + -0x10) * 0xfc +
                           *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x3a4)) != 2)) &&
                 (*(int *)(*(int *)(unaff_EBP + -0x10) * 0xfc +
                          *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x3a4)) != 6)) &&
                (*(int *)(*(int *)(unaff_EBP + -0x10) * 0xfc +
                         *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x3a4)) != 7)))) {
              *(undefined4 *)(unaff_EBP + -0x34) = 1;
              *(undefined4 *)(unaff_EBP + -0x18) = *(undefined4 *)(unaff_EBP + -0x40);
              pvVar2 = operator_new(0x50);
              *(void **)(unaff_EBP + -0x70) = pvVar2;
              *(undefined4 *)(unaff_EBP + -4) = 2;
              if (*(int *)(unaff_EBP + -0x70) == 0) {
                *(undefined4 *)(unaff_EBP + -0x90) = 0;
              }
              else {
                puVar3 = FUN_0043ab51(*(undefined4 **)(unaff_EBP + -0x70));
                *(undefined4 **)(unaff_EBP + -0x90) = puVar3;
              }
              *(undefined4 *)(unaff_EBP + -0x6c) = *(undefined4 *)(unaff_EBP + -0x90);
              *(undefined4 *)(unaff_EBP + -4) = 0xffffffff;
              *(undefined4 *)
               (*(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16d0) +
               *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16c8) * 4) =
                   *(undefined4 *)(unaff_EBP + -0x6c);
              *(undefined4 *)
               (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16d0) +
                        *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16c8) * 4) + 0x4c) =
                   *(undefined4 *)(*(int *)(unaff_EBP + -0x7c) + 0x16c8);
              if (*(int *)(*(int *)(unaff_EBP + -0x10) * 0xfc +
                           *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x3a4) + 0x1c +
                          *(int *)(unaff_EBP + -0x40) * 4) == -2) {
                *(undefined4 *)
                 (*(int *)(unaff_EBP + -0x38) * 0xfc + *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x3a4)
                 ) = 0xb;
                *(undefined4 *)
                 (*(int *)(*(int *)(unaff_EBP + -0x7c) + 0x3a4) + 0x14 +
                 *(int *)(unaff_EBP + -0x38) * 0xfc) = 0;
              }
              else {
                *(undefined4 *)
                 (*(int *)(unaff_EBP + -0x38) * 0xfc + *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x3a4)
                 ) = 10;
                *(undefined4 *)
                 (*(int *)(*(int *)(unaff_EBP + -0x7c) + 0x3a4) + 0x14 +
                 *(int *)(unaff_EBP + -0x38) * 0xfc) = 1;
              }
              *(undefined4 *)
               (*(int *)(*(int *)(unaff_EBP + -0x7c) + 0x3a4) + 0x18 +
               *(int *)(unaff_EBP + -0x38) * 0xfc) = 0;
              iVar5 = *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x3a4) +
                      *(int *)(unaff_EBP + -0x10) * 0xfc;
              uVar1 = *(undefined4 *)(iVar5 + 0x70 + *(int *)(unaff_EBP + -0x40) * 8);
              iVar6 = *(int *)(unaff_EBP + -0x38) * 0xfc;
              iVar4 = *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x3a4);
              *(undefined4 *)(iVar4 + 0xc0 + iVar6) =
                   *(undefined4 *)(iVar5 + 0x6c + *(int *)(unaff_EBP + -0x40) * 8);
              *(undefined4 *)(iVar4 + 0xc4 + iVar6) = uVar1;
              *(int *)(*(int *)(*(int *)(unaff_EBP + -0x7c) + 0x3a4) + 0xc0 +
                      *(int *)(unaff_EBP + -0x38) * 0xfc) =
                   *(int *)(*(int *)(*(int *)(unaff_EBP + -0x7c) + 0x3a4) + 0xc0 +
                           *(int *)(unaff_EBP + -0x38) * 0xfc) + -5;
              *(undefined4 *)
               (*(int *)(*(int *)(unaff_EBP + -0x7c) + 0x3a4) + 0xe0 +
               *(int *)(unaff_EBP + -0x38) * 0xfc) =
                   *(undefined4 *)(*(int *)(unaff_EBP + -0x7c) + 0x16c8);
              *(undefined4 *)
               (*(int *)(unaff_EBP + -0x10) * 0xfc + *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x3a4) +
                0x1c + *(int *)(unaff_EBP + -0x40) * 4) = *(undefined4 *)(unaff_EBP + -0x38);
              *(undefined4 *)
               (*(int *)(unaff_EBP + -0x10) * 0xfc + *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x3a4) +
                0xe4 + *(int *)(unaff_EBP + -0x40) * 4) =
                   *(undefined4 *)(*(int *)(unaff_EBP + -0x7c) + 0x16c8);
              **(undefined4 **)
                (*(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16d0) +
                *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16c8) * 4) = 0;
              *(undefined4 *)
               (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16d0) +
                        *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16c8) * 4) + 4) =
                   *(undefined4 *)(unaff_EBP + -0x10);
              *(undefined4 *)
               (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16d0) +
                        *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16c8) * 4) + 8) =
                   *(undefined4 *)(unaff_EBP + -0x40);
              *(undefined4 *)
               (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16d0) +
                        *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16c8) * 4) + 0x14) = 1;
              *(undefined4 *)
               (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16d0) +
                        *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16c8) * 4) + 0x18) =
                   *(undefined4 *)(unaff_EBP + -0x38);
              *(undefined4 *)
               (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16d0) +
                        *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16c8) * 4) + 0x38) =
                   *(undefined4 *)(unaff_EBP + -0x38);
              iVar4 = *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x3a4) +
                      *(int *)(unaff_EBP + -0x10) * 0xfc;
              uVar1 = *(undefined4 *)(iVar4 + 0x70 + *(int *)(unaff_EBP + -0x40) * 8);
              *(undefined4 *)(unaff_EBP + -0x28) =
                   *(undefined4 *)(iVar4 + 0x6c + *(int *)(unaff_EBP + -0x40) * 8);
              *(undefined4 *)(unaff_EBP + -0x24) = uVar1;
              FUN_0043ac51(*(void **)(*(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16d0) +
                                     *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16c8) * 4),
                           *(int *)(unaff_EBP + -0x28),*(int *)(unaff_EBP + -0x24));
              *(int *)(unaff_EBP + -0x28) = *(int *)(unaff_EBP + -0x28) + -6;
              FUN_0043ac51(*(void **)(*(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16d0) +
                                     *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16c8) * 4),
                           *(int *)(unaff_EBP + -0x28),*(int *)(unaff_EBP + -0x24));
              *(undefined4 *)(unaff_EBP + -0x2c) =
                   *(undefined4 *)(*(int *)(unaff_EBP + -0x7c) + 0x16c8);
              *(undefined4 *)(unaff_EBP + -0x20) = *(undefined4 *)(unaff_EBP + -0x38);
              iVar5 = *(int *)(unaff_EBP + -0x38) * 0xfc;
              iVar4 = *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x3a4);
              FUN_00425f03(*(HDC *)(unaff_EBP + 8),
                           (int *)(*(int *)(unaff_EBP + -0x38) * 0xfc +
                                  *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x3a4)),
                           *(int *)(iVar4 + 0xc0 + iVar5),*(int *)(iVar4 + 0xc4 + iVar5),1);
              FUN_004286f7(*(void **)(unaff_EBP + -0x7c),
                           *(int **)(*(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16d0) +
                                    *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16c8) * 4),
                           *(HDC *)(unaff_EBP + 8));
              *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16c8) =
                   *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16c8) + 1;
              *(int *)(unaff_EBP + -0x38) = *(int *)(unaff_EBP + -0x38) + 1;
            }
            else {
              pvVar2 = operator_new(0x50);
              *(void **)(unaff_EBP + -0x78) = pvVar2;
              *(undefined4 *)(unaff_EBP + -4) = 3;
              if (*(int *)(unaff_EBP + -0x78) == 0) {
                *(undefined4 *)(unaff_EBP + -0x94) = 0;
              }
              else {
                puVar3 = FUN_0043ab51(*(undefined4 **)(unaff_EBP + -0x78));
                *(undefined4 **)(unaff_EBP + -0x94) = puVar3;
              }
              *(undefined4 *)(unaff_EBP + -0x74) = *(undefined4 *)(unaff_EBP + -0x94);
              *(undefined4 *)(unaff_EBP + -4) = 0xffffffff;
              *(undefined4 *)
               (*(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16d0) +
               *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16c8) * 4) =
                   *(undefined4 *)(unaff_EBP + -0x74);
              *(undefined4 *)
               (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16d0) +
                        *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16c8) * 4) + 0x4c) =
                   *(undefined4 *)(*(int *)(unaff_EBP + -0x7c) + 0x16c8);
              *(undefined4 *)
               (*(int *)(unaff_EBP + -0x10) * 0xfc + *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x3a4) +
                0x1c + *(int *)(unaff_EBP + -0x40) * 4) = *(undefined4 *)(unaff_EBP + -0x20);
              *(undefined4 *)
               (*(int *)(unaff_EBP + -0x10) * 0xfc + *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x3a4) +
                0xe4 + *(int *)(unaff_EBP + -0x40) * 4) =
                   *(undefined4 *)(*(int *)(unaff_EBP + -0x7c) + 0x16c8);
              **(undefined4 **)
                (*(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16d0) +
                *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16c8) * 4) = 0;
              *(undefined4 *)
               (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16d0) +
                        *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16c8) * 4) + 4) =
                   *(undefined4 *)(unaff_EBP + -0x10);
              *(undefined4 *)
               (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16d0) +
                        *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16c8) * 4) + 8) =
                   *(undefined4 *)(unaff_EBP + -0x40);
              *(undefined4 *)
               (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16d0) +
                        *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16c8) * 4) + 0x14) = 2;
              *(undefined4 *)
               (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16d0) +
                        *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16c8) * 4) + 0x20) =
                   *(undefined4 *)(unaff_EBP + -0x2c);
              *(undefined4 *)
               (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16d0) +
                        *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16c8) * 4) + 0x38) =
                   *(undefined4 *)(unaff_EBP + -0x20);
              iVar4 = *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x3a4) +
                      *(int *)(unaff_EBP + -0x10) * 0xfc;
              uVar1 = *(undefined4 *)(iVar4 + 0x70 + *(int *)(unaff_EBP + -0x40) * 8);
              *(undefined4 *)(unaff_EBP + -0x28) =
                   *(undefined4 *)(iVar4 + 0x6c + *(int *)(unaff_EBP + -0x40) * 8);
              *(undefined4 *)(unaff_EBP + -0x24) = uVar1;
              FUN_0043ac51(*(void **)(*(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16d0) +
                                     *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16c8) * 4),
                           *(int *)(unaff_EBP + -0x28),*(int *)(unaff_EBP + -0x24));
              *(int *)(unaff_EBP + -0x28) = *(int *)(unaff_EBP + -0x28) + -6;
              FUN_0043ac51(*(void **)(*(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16d0) +
                                     *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16c8) * 4),
                           *(int *)(unaff_EBP + -0x28),*(int *)(unaff_EBP + -0x24));
              iVar4 = *(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16d0) +
                                       *(int *)(unaff_EBP + -0x2c) * 4) + 0x2c);
              uVar1 = *(undefined4 *)(iVar4 + 0x18);
              *(undefined4 *)(unaff_EBP + -0x28) = *(undefined4 *)(iVar4 + 0x14);
              *(undefined4 *)(unaff_EBP + -0x24) = uVar1;
              FUN_0043ac51(*(void **)(*(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16d0) +
                                     *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16c8) * 4),
                           *(int *)(unaff_EBP + -0x28),*(int *)(unaff_EBP + -0x24));
              FUN_0043ae30(*(void **)(*(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16d0) +
                                     *(int *)(unaff_EBP + -0x2c) * 4),*(int *)(unaff_EBP + -0x28),
                           *(int *)(unaff_EBP + -0x24),
                           *(undefined4 *)(*(int *)(unaff_EBP + -0x7c) + 0x16c8),2,0);
              *(undefined4 *)(unaff_EBP + -0x2c) =
                   *(undefined4 *)(*(int *)(unaff_EBP + -0x7c) + 0x16c8);
              FUN_004286f7(*(void **)(unaff_EBP + -0x7c),
                           *(int **)(*(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16d0) +
                                    *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16c8) * 4),
                           *(HDC *)(unaff_EBP + 8));
              *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16c8) =
                   *(int *)(*(int *)(unaff_EBP + -0x7c) + 0x16c8) + 1;
            }
          }
          *(int *)(unaff_EBP + -0x40) = *(int *)(unaff_EBP + -0x40) + 1;
        }
      }
      *(int *)(unaff_EBP + -0x10) = *(int *)(unaff_EBP + -0x10) + 1;
    }
    uVar1 = 1;
  }
  ExceptionList = *(void **)(unaff_EBP + -0xc);
  return uVar1;
}
