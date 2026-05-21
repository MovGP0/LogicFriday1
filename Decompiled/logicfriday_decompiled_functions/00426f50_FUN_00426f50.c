/* 00426f50 FUN_00426f50 */

void FUN_00426f50(void)

{
  void *pvVar1;
  undefined4 *puVar2;
  undefined4 uVar3;
  undefined4 extraout_ECX;
  int iVar4;
  int unaff_EBP;
  
  FUN_0043f30c();
  *(undefined4 *)(unaff_EBP + -0x5c) = extraout_ECX;
  *(undefined4 *)(*(int *)(unaff_EBP + -0x5c) + 0x26ec) = 0;
  *(undefined4 *)(unaff_EBP + -0x1c) = 1;
  while (*(int *)(unaff_EBP + -0x1c) < *(int *)(*(int *)(unaff_EBP + -0x5c) + 0x2674)) {
    *(int *)(unaff_EBP + -0x50) = *(int *)(*(int *)(unaff_EBP + -0x5c) + 0x2670) + -1;
    while (-1 < *(int *)(unaff_EBP + -0x50)) {
      *(undefined4 *)(unaff_EBP + -0x48) =
           *(undefined4 *)
            (*(int *)(unaff_EBP + -0x1c) * 0x48 +
            *(int *)(*(int *)(*(int *)(unaff_EBP + -0x5c) + 0x2678) +
                    *(int *)(unaff_EBP + -0x50) * 4));
      if ((*(int *)(unaff_EBP + -0x48) != -1) &&
         (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x5c) + 0x3a4) + 0xbc +
                  *(int *)(unaff_EBP + -0x48) * 0xfc) != 0)) {
        FUN_00428eb1(*(void **)(unaff_EBP + -0x5c),*(int *)(unaff_EBP + -0x50),
                     *(int *)(unaff_EBP + -0x50),*(int *)(unaff_EBP + -0x1c) + -1,1,1);
        FUN_00428eb1(*(void **)(unaff_EBP + -0x5c),*(int *)(unaff_EBP + -0x50),
                     *(int *)(unaff_EBP + -0x50),*(int *)(unaff_EBP + -0x1c) + -1,1,1);
      }
      *(int *)(unaff_EBP + -0x50) = *(int *)(unaff_EBP + -0x50) + -1;
    }
    *(int *)(unaff_EBP + -0x1c) = *(int *)(unaff_EBP + -0x1c) + 1;
  }
  *(undefined4 *)(unaff_EBP + -0x1c) = 1;
  do {
    if (*(int *)(*(int *)(unaff_EBP + -0x5c) + 0x2674) <= *(int *)(unaff_EBP + -0x1c)) {
      ExceptionList = *(void **)(unaff_EBP + -0xc);
      return;
    }
    *(int *)(unaff_EBP + -0x50) = *(int *)(*(int *)(unaff_EBP + -0x5c) + 0x2670) + -1;
    while (-1 < *(int *)(unaff_EBP + -0x50)) {
      *(undefined4 *)(unaff_EBP + -0x48) =
           *(undefined4 *)
            (*(int *)(unaff_EBP + -0x1c) * 0x48 +
            *(int *)(*(int *)(*(int *)(unaff_EBP + -0x5c) + 0x2678) +
                    *(int *)(unaff_EBP + -0x50) * 4));
      if (*(int *)(unaff_EBP + -0x48) != -1) {
        *(undefined4 *)(unaff_EBP + -0x34) =
             *(undefined4 *)
              (*(int *)(*(int *)(unaff_EBP + -0x5c) + 0x3a4) + 0x3c +
              *(int *)(unaff_EBP + -0x48) * 0xfc);
        *(undefined4 *)(unaff_EBP + -0x44) =
             *(undefined4 *)
              (*(int *)(*(int *)(unaff_EBP + -0x5c) + 0x3a4) + 0x40 +
              *(int *)(unaff_EBP + -0x48) * 0xfc);
        *(undefined4 *)(unaff_EBP + -0x3c) = 0;
        while (*(int *)(unaff_EBP + -0x3c) <
               *(int *)(*(int *)(*(int *)(unaff_EBP + -0x5c) + 0x3a4) + 0x18 +
                       *(int *)(unaff_EBP + -0x48) * 0xfc)) {
          if ((*(int *)(*(int *)(unaff_EBP + -0x48) * 0xfc +
                        *(int *)(*(int *)(unaff_EBP + -0x5c) + 0x3a4) + 0x1c +
                       *(int *)(unaff_EBP + -0x3c) * 4) != -2) &&
             (*(int *)(*(int *)(unaff_EBP + -0x48) * 0xfc +
                       *(int *)(*(int *)(unaff_EBP + -0x5c) + 0x3a4) + 0x1c +
                      *(int *)(unaff_EBP + -0x3c) * 4) != -1)) {
            *(undefined4 *)(unaff_EBP + -0x4c) =
                 *(undefined4 *)
                  (*(int *)(unaff_EBP + -0x48) * 0xfc +
                   *(int *)(*(int *)(unaff_EBP + -0x5c) + 0x3a4) + 0x1c +
                  *(int *)(unaff_EBP + -0x3c) * 4);
            *(undefined4 *)(unaff_EBP + -0x24) =
                 *(undefined4 *)
                  (*(int *)(*(int *)(unaff_EBP + -0x5c) + 0x3a4) + 0x3c +
                  *(int *)(unaff_EBP + -0x4c) * 0xfc);
            *(undefined4 *)(unaff_EBP + -0x18) =
                 *(undefined4 *)
                  (*(int *)(*(int *)(unaff_EBP + -0x5c) + 0x3a4) + 0x40 +
                  *(int *)(unaff_EBP + -0x4c) * 0xfc);
            if (*(uint *)(*(int *)(unaff_EBP + -0x5c) + 0x16c0) <=
                *(uint *)(*(int *)(unaff_EBP + -0x5c) + 0x16c8)) {
              *(int *)(*(int *)(unaff_EBP + -0x5c) + 0x16c0) =
                   *(int *)(*(int *)(unaff_EBP + -0x5c) + 0x16c0) + 100;
              pvVar1 = _realloc(*(void **)(*(int *)(unaff_EBP + -0x5c) + 0x16d0),
                                *(int *)(*(int *)(unaff_EBP + -0x5c) + 0x16c0) << 2);
              *(void **)(*(int *)(unaff_EBP + -0x5c) + 0x16d0) = pvVar1;
            }
            pvVar1 = operator_new(0x50);
            *(void **)(unaff_EBP + -0x58) = pvVar1;
            *(undefined4 *)(unaff_EBP + -4) = 0;
            if (*(int *)(unaff_EBP + -0x58) == 0) {
              *(undefined4 *)(unaff_EBP + -0x60) = 0;
            }
            else {
              puVar2 = FUN_0043ab51(*(undefined4 **)(unaff_EBP + -0x58));
              *(undefined4 **)(unaff_EBP + -0x60) = puVar2;
            }
            *(undefined4 *)(unaff_EBP + -0x54) = *(undefined4 *)(unaff_EBP + -0x60);
            *(undefined4 *)(unaff_EBP + -4) = 0xffffffff;
            *(undefined4 *)
             (*(int *)(*(int *)(unaff_EBP + -0x5c) + 0x16d0) +
             *(int *)(*(int *)(unaff_EBP + -0x5c) + 0x16c8) * 4) =
                 *(undefined4 *)(unaff_EBP + -0x54);
            *(undefined4 *)
             (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x5c) + 0x16d0) +
                      *(int *)(*(int *)(unaff_EBP + -0x5c) + 0x16c8) * 4) + 0x4c) =
                 *(undefined4 *)(*(int *)(unaff_EBP + -0x5c) + 0x16c8);
            **(undefined4 **)
              (*(int *)(*(int *)(unaff_EBP + -0x5c) + 0x16d0) +
              *(int *)(*(int *)(unaff_EBP + -0x5c) + 0x16c8) * 4) = 0;
            *(undefined4 *)
             (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x5c) + 0x16d0) +
                      *(int *)(*(int *)(unaff_EBP + -0x5c) + 0x16c8) * 4) + 4) =
                 *(undefined4 *)(unaff_EBP + -0x48);
            *(undefined4 *)
             (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x5c) + 0x16d0) +
                      *(int *)(*(int *)(unaff_EBP + -0x5c) + 0x16c8) * 4) + 8) =
                 *(undefined4 *)(unaff_EBP + -0x3c);
            *(undefined4 *)
             (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x5c) + 0x16d0) +
                      *(int *)(*(int *)(unaff_EBP + -0x5c) + 0x16c8) * 4) + 0x38) =
                 *(undefined4 *)(unaff_EBP + -0x4c);
            *(undefined4 *)
             (*(int *)(unaff_EBP + -0x48) * 0xfc + *(int *)(*(int *)(unaff_EBP + -0x5c) + 0x3a4) +
              0xe4 + *(int *)(unaff_EBP + -0x3c) * 4) =
                 *(undefined4 *)(*(int *)(unaff_EBP + -0x5c) + 0x16c8);
            *(int *)(*(int *)(unaff_EBP + -0x5c) + 0x16c8) =
                 *(int *)(*(int *)(unaff_EBP + -0x5c) + 0x16c8) + 1;
            iVar4 = *(int *)(*(int *)(unaff_EBP + -0x5c) + 0x3a4) +
                    *(int *)(unaff_EBP + -0x48) * 0xfc;
            uVar3 = FUN_00427686(*(void **)(unaff_EBP + -0x5c),*(HDC *)(unaff_EBP + 8),
                                 *(int *)(unaff_EBP + -0x4c),
                                 *(int *)(iVar4 + 0x6c + *(int *)(unaff_EBP + -0x3c) * 8),
                                 *(int *)(iVar4 + 0x70 + *(int *)(unaff_EBP + -0x3c) * 8),
                                 *(int *)(unaff_EBP + -0x44),*(int *)(unaff_EBP + -0x34) + -1);
            *(undefined4 *)(unaff_EBP + -0x38) = uVar3;
            if (*(int *)(unaff_EBP + -0x38) == 0) {
              iVar4 = *(int *)(*(int *)(unaff_EBP + -0x5c) + 0x3a4) +
                      *(int *)(unaff_EBP + -0x48) * 0xfc;
              FUN_0043ac51(*(void **)(*(int *)(*(int *)(unaff_EBP + -0x5c) + 0x16d0) + -4 +
                                     *(int *)(*(int *)(unaff_EBP + -0x5c) + 0x16c8) * 4),
                           *(int *)(iVar4 + 0x6c + *(int *)(unaff_EBP + -0x3c) * 8),
                           *(int *)(iVar4 + 0x70 + *(int *)(unaff_EBP + -0x3c) * 8));
              iVar4 = FUN_00428eb1(*(void **)(unaff_EBP + -0x5c),*(int *)(unaff_EBP + -0x44),
                                   *(int *)(unaff_EBP + -0x18),*(int *)(unaff_EBP + -0x34) + -1,1,1)
              ;
              *(int *)(unaff_EBP + -0x40) = iVar4;
              iVar4 = *(int *)(*(int *)(unaff_EBP + -0x5c) + 0x3a4) +
                      *(int *)(unaff_EBP + -0x48) * 0xfc;
              uVar3 = *(undefined4 *)(iVar4 + 0x70 + *(int *)(unaff_EBP + -0x3c) * 8);
              *(undefined4 *)(unaff_EBP + -0x14) =
                   *(undefined4 *)(iVar4 + 0x6c + *(int *)(unaff_EBP + -0x3c) * 8);
              *(undefined4 *)(unaff_EBP + -0x10) = uVar3;
              *(int *)(unaff_EBP + -0x2c) =
                   *(int *)(unaff_EBP + -0x14) + *(int *)(unaff_EBP + -0x40) * -0xf;
              *(undefined4 *)(unaff_EBP + -0x28) = *(undefined4 *)(unaff_EBP + -0x10);
              MoveToEx(*(HDC *)(unaff_EBP + 8),*(int *)(unaff_EBP + -0x14),
                       *(int *)(unaff_EBP + -0x10),(LPPOINT)0x0);
              LineTo(*(HDC *)(unaff_EBP + 8),*(int *)(unaff_EBP + -0x2c),*(int *)(unaff_EBP + -0x28)
                    );
              FUN_0043ac51(*(void **)(*(int *)(*(int *)(unaff_EBP + -0x5c) + 0x16d0) + -4 +
                                     *(int *)(*(int *)(unaff_EBP + -0x5c) + 0x16c8) * 4),
                           *(int *)(unaff_EBP + -0x2c),*(int *)(unaff_EBP + -0x28));
              if (*(int *)(unaff_EBP + -0x44) < *(int *)(unaff_EBP + -0x18)) {
                *(int *)(unaff_EBP + -0x20) = *(int *)(unaff_EBP + -0x18) + -1;
              }
              else {
                *(undefined4 *)(unaff_EBP + -0x20) = *(undefined4 *)(unaff_EBP + -0x18);
              }
              iVar4 = FUN_00429288(*(void **)(unaff_EBP + -0x5c),*(int *)(unaff_EBP + -0x20),
                                   *(int *)(unaff_EBP + -0x34) + -1,*(int *)(unaff_EBP + -0x24),1);
              *(int *)(unaff_EBP + -0x40) = iVar4;
              if (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x5c) + 0x3a4) + 0xb8 +
                          *(int *)(unaff_EBP + -0x4c) * 0xfc) == 0) {
                *(undefined4 *)(unaff_EBP + -0x14) = *(undefined4 *)(unaff_EBP + -0x2c);
                *(undefined4 *)(unaff_EBP + -0x10) = *(undefined4 *)(unaff_EBP + -0x28);
                *(int *)(unaff_EBP + -0x28) =
                     *(int *)(unaff_EBP + -0x40) * 0xf +
                     *(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x5c) + 0x2678) +
                                      *(int *)(unaff_EBP + -0x20) * 4) + 0x24);
                LineTo(*(HDC *)(unaff_EBP + 8),*(int *)(unaff_EBP + -0x2c),
                       *(int *)(unaff_EBP + -0x28));
                FUN_0043ac51(*(void **)(*(int *)(*(int *)(unaff_EBP + -0x5c) + 0x16d0) + -4 +
                                       *(int *)(*(int *)(unaff_EBP + -0x5c) + 0x16c8) * 4),
                             *(int *)(unaff_EBP + -0x2c),*(int *)(unaff_EBP + -0x28));
                *(undefined4 *)(unaff_EBP + -0x14) = *(undefined4 *)(unaff_EBP + -0x2c);
                *(undefined4 *)(unaff_EBP + -0x10) = *(undefined4 *)(unaff_EBP + -0x28);
                *(undefined4 *)(unaff_EBP + -0x2c) =
                     *(undefined4 *)
                      (**(int **)(*(int *)(unaff_EBP + -0x5c) + 0x2678) + 0x1c +
                      (*(int *)(unaff_EBP + -0x24) + 1) * 0x48);
                *(undefined4 *)(unaff_EBP + -0x28) = *(undefined4 *)(unaff_EBP + -0x10);
                LineTo(*(HDC *)(unaff_EBP + 8),*(int *)(unaff_EBP + -0x2c),
                       *(int *)(unaff_EBP + -0x28));
                uVar3 = FUN_00427686(*(void **)(unaff_EBP + -0x5c),*(HDC *)(unaff_EBP + 8),
                                     *(int *)(unaff_EBP + -0x4c),*(int *)(unaff_EBP + -0x2c),
                                     *(int *)(unaff_EBP + -0x28),*(int *)(unaff_EBP + -0x20),
                                     *(int *)(unaff_EBP + -0x24));
                *(undefined4 *)(unaff_EBP + -0x38) = uVar3;
              }
              else {
                *(undefined4 *)(unaff_EBP + -0x14) = *(undefined4 *)(unaff_EBP + -0x2c);
                *(undefined4 *)(unaff_EBP + -0x10) = *(undefined4 *)(unaff_EBP + -0x28);
                *(int *)(unaff_EBP + -0x28) =
                     *(int *)(unaff_EBP + -0x40) * 0xf +
                     *(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x5c) + 0x2678) +
                                      *(int *)(unaff_EBP + -0x20) * 4) + 0x24);
                LineTo(*(HDC *)(unaff_EBP + 8),*(int *)(unaff_EBP + -0x2c),
                       *(int *)(unaff_EBP + -0x28));
                uVar3 = FUN_00427686(*(void **)(unaff_EBP + -0x5c),*(HDC *)(unaff_EBP + 8),
                                     *(int *)(unaff_EBP + -0x4c),*(int *)(unaff_EBP + -0x2c),
                                     *(int *)(unaff_EBP + -0x28),*(int *)(unaff_EBP + -0x20),
                                     *(int *)(unaff_EBP + -0x34) + -1);
                *(undefined4 *)(unaff_EBP + -0x38) = uVar3;
                if (*(int *)(unaff_EBP + -0x38) == 0) {
                  FUN_0043ac51(*(void **)(*(int *)(*(int *)(unaff_EBP + -0x5c) + 0x16d0) + -4 +
                                         *(int *)(*(int *)(unaff_EBP + -0x5c) + 0x16c8) * 4),
                               *(int *)(unaff_EBP + -0x2c),*(int *)(unaff_EBP + -0x28));
                  *(undefined4 *)(unaff_EBP + -0x14) = *(undefined4 *)(unaff_EBP + -0x2c);
                  *(undefined4 *)(unaff_EBP + -0x10) = *(undefined4 *)(unaff_EBP + -0x28);
                  *(int *)(unaff_EBP + -0x30) = *(int *)(unaff_EBP + -0x34) + -1;
                  while (*(int *)(unaff_EBP + -0x24) < *(int *)(unaff_EBP + -0x30)) {
                    *(undefined4 *)(unaff_EBP + -0x2c) =
                         *(undefined4 *)
                          (**(int **)(*(int *)(unaff_EBP + -0x5c) + 0x2678) + 0x1c +
                          *(int *)(unaff_EBP + -0x30) * 0x48);
                    LineTo(*(HDC *)(unaff_EBP + 8),*(int *)(unaff_EBP + -0x2c),
                           *(int *)(unaff_EBP + -0x28));
                    uVar3 = FUN_00427686(*(void **)(unaff_EBP + -0x5c),*(HDC *)(unaff_EBP + 8),
                                         *(int *)(unaff_EBP + -0x4c),*(int *)(unaff_EBP + -0x2c),
                                         *(int *)(unaff_EBP + -0x28),*(int *)(unaff_EBP + -0x20),
                                         *(int *)(unaff_EBP + -0x30) + -1);
                    *(undefined4 *)(unaff_EBP + -0x38) = uVar3;
                    if (*(int *)(unaff_EBP + -0x38) != 0) break;
                    if (*(int *)(unaff_EBP + -0x30) == *(int *)(unaff_EBP + -0x24) + 1) {
                      FUN_0043ac51(*(void **)(*(int *)(*(int *)(unaff_EBP + -0x5c) + 0x16d0) + -4 +
                                             *(int *)(*(int *)(unaff_EBP + -0x5c) + 0x16c8) * 4),
                                   *(int *)(unaff_EBP + -0x2c),*(int *)(unaff_EBP + -0x28));
                    }
                    *(int *)(unaff_EBP + -0x30) = *(int *)(unaff_EBP + -0x30) + -1;
                  }
                  if (*(int *)(unaff_EBP + -0x38) == 0) {
                    iVar4 = FUN_00428eb1(*(void **)(unaff_EBP + -0x5c),*(int *)(unaff_EBP + -0x20),
                                         *(int *)(unaff_EBP + -0x18),*(int *)(unaff_EBP + -0x24),0,1
                                        );
                    *(int *)(unaff_EBP + -0x40) = iVar4;
                    *(undefined4 *)(unaff_EBP + -0x14) = *(undefined4 *)(unaff_EBP + -0x2c);
                    *(undefined4 *)(unaff_EBP + -0x10) = *(undefined4 *)(unaff_EBP + -0x28);
                    *(undefined4 *)(unaff_EBP + -0x28) =
                         *(undefined4 *)
                          (*(int *)(*(int *)(unaff_EBP + -0x5c) + 0x3a4) + 0xb0 +
                          *(int *)(unaff_EBP + -0x4c) * 0xfc);
                    LineTo(*(HDC *)(unaff_EBP + 8),*(int *)(unaff_EBP + -0x2c),
                           *(int *)(unaff_EBP + -0x28));
                    uVar3 = FUN_00427686(*(void **)(unaff_EBP + -0x5c),*(HDC *)(unaff_EBP + 8),
                                         *(int *)(unaff_EBP + -0x4c),*(int *)(unaff_EBP + -0x2c),
                                         *(int *)(unaff_EBP + -0x28),*(int *)(unaff_EBP + -0x20),
                                         *(int *)(unaff_EBP + -0x24));
                    *(undefined4 *)(unaff_EBP + -0x38) = uVar3;
                  }
                }
              }
            }
          }
          *(int *)(unaff_EBP + -0x3c) = *(int *)(unaff_EBP + -0x3c) + 1;
        }
      }
      *(int *)(unaff_EBP + -0x50) = *(int *)(unaff_EBP + -0x50) + -1;
    }
    *(int *)(unaff_EBP + -0x1c) = *(int *)(unaff_EBP + -0x1c) + 1;
  } while( true );
}
