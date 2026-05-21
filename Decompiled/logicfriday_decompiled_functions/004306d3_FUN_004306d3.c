/* 004306d3 FUN_004306d3 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

undefined4 FUN_004306d3(void)

{
  undefined4 uVar1;
  size_t sVar2;
  void *pvVar3;
  undefined4 *puVar4;
  undefined4 extraout_ECX;
  int iVar5;
  int unaff_EBP;
  char *pcVar6;
  
  FUN_0043f30c();
  *(uint *)(unaff_EBP + -0x24) = DAT_00451a00 ^ *(uint *)(unaff_EBP + 4);
  *(undefined4 *)(unaff_EBP + -0x68) = extraout_ECX;
  *(undefined4 *)(unaff_EBP + -0x54) = 0;
  pcVar6 = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
  puVar4 = (undefined4 *)(unaff_EBP + -0x40);
  for (iVar5 = 6; iVar5 != 0; iVar5 = iVar5 + -1) {
    *puVar4 = *(undefined4 *)pcVar6;
    pcVar6 = pcVar6 + 4;
    puVar4 = puVar4 + 1;
  }
  *(undefined2 *)puVar4 = *(undefined2 *)pcVar6;
  *(char *)((int)puVar4 + 2) = pcVar6[2];
  *(undefined4 *)(unaff_EBP + -0x18) = 0;
  do {
    if (*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16c4) <= *(int *)(unaff_EBP + -0x18)) {
      *(undefined4 *)(unaff_EBP + -0x10) = 0;
      *(undefined4 *)(unaff_EBP + -0x18) = 0;
      while (*(int *)(unaff_EBP + -0x18) < *(int *)(*(int *)(unaff_EBP + -0x68) + 0x16c8)) {
        if ((*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                              *(int *)(unaff_EBP + -0x18) * 4) + 0x44) != 0) &&
           (*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                             *(int *)(unaff_EBP + -0x18) * 4) + 0x40) == 0)) {
          *(int *)(unaff_EBP + -0x54) = *(int *)(unaff_EBP + -0x54) + 1;
          *(undefined4 *)(unaff_EBP + -0x10) = 1;
          if ((((*(int *)(*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                           *(int *)(unaff_EBP + -0x18) * 4) + 0x2c) + 8) == 0) ||
               (*(int *)(*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                          *(int *)(unaff_EBP + -0x18) * 4) + 0x2c) + 8 +
                        (*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                          *(int *)(unaff_EBP + -0x18) * 4) + 0x28) + -2) * 0x14) ==
                0)) && ((*(int *)(*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                                   *(int *)(unaff_EBP + -0x18) * 4) + 0x2c) + 8) ==
                         0 || (*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                                *(int *)(unaff_EBP + -0x18) * 4) + 0x14) != -3))))
             && ((*(int *)(*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                            *(int *)(unaff_EBP + -0x18) * 4) + 0x2c) + 8 +
                          (*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                            *(int *)(unaff_EBP + -0x18) * 4) + 0x28) + -2) * 0x14)
                  == 0 || (**(int **)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                     *(int *)(unaff_EBP + -0x18) * 4) != -3)))) {
            if ((*(int *)(*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                           *(int *)(unaff_EBP + -0x18) * 4) + 0x2c) + 8) == 0) &&
               (*(int *)(*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                          *(int *)(unaff_EBP + -0x18) * 4) + 0x2c) + 8 +
                        (*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                          *(int *)(unaff_EBP + -0x18) * 4) + 0x28) + -2) * 0x14) ==
                0)) {
              pvVar3 = operator_new(0x50);
              *(void **)(unaff_EBP + -100) = pvVar3;
              *(undefined4 *)(unaff_EBP + -4) = 0;
              if (*(int *)(unaff_EBP + -100) == 0) {
                *(undefined4 *)(unaff_EBP + -0x6c) = 0;
              }
              else {
                puVar4 = FUN_0043ab51(*(undefined4 **)(unaff_EBP + -100));
                *(undefined4 **)(unaff_EBP + -0x6c) = puVar4;
              }
              *(undefined4 *)(unaff_EBP + -0x60) = *(undefined4 *)(unaff_EBP + -0x6c);
              *(undefined4 *)(unaff_EBP + -4) = 0xffffffff;
              *(undefined4 *)
               (*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
               *(int *)(*(int *)(unaff_EBP + -0x68) + 0x16c8) * 4) =
                   *(undefined4 *)(unaff_EBP + -0x60);
              *(undefined4 *)
               (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                        *(int *)(*(int *)(unaff_EBP + -0x68) + 0x16c8) * 4) + 0x4c) =
                   *(undefined4 *)(*(int *)(unaff_EBP + -0x68) + 0x16c8);
              *(int *)(*(int *)(unaff_EBP + -0x68) + 0x16c8) =
                   *(int *)(*(int *)(unaff_EBP + -0x68) + 0x16c8) + 1;
              FUN_0043dc5c(*(void **)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                     *(int *)(unaff_EBP + -0x18) * 4),
                           *(int **)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) + -4 +
                                    *(int *)(*(int *)(unaff_EBP + -0x68) + 0x16c8) * 4),
                           *(HWND *)(unaff_EBP + 8));
              *(undefined4 *)(unaff_EBP + -0x5c) = 0;
              while (*(int *)(unaff_EBP + -0x5c) <
                     *(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) + -4 +
                                      *(int *)(*(int *)(unaff_EBP + -0x68) + 0x16c8) * 4) + 0x30)) {
                *(undefined4 *)(unaff_EBP + -0x20) =
                     *(undefined4 *)
                      (*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) + -4 +
                                        *(int *)(*(int *)(unaff_EBP + -0x68) + 0x16c8) * 4) + 0x34)
                       + 0xc + *(int *)(unaff_EBP + -0x5c) * 0x14);
                if (*(int *)(*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) + -4 +
                                              *(int *)(*(int *)(unaff_EBP + -0x68) + 0x16c8) * 4) +
                                     0x34) + 0x10 + *(int *)(unaff_EBP + -0x5c) * 0x14) == 0) {
                  *(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                   *(int *)(unaff_EBP + -0x20) * 4) + 0xc) =
                       *(int *)(*(int *)(unaff_EBP + -0x68) + 0x16c8) + -1;
                }
                else {
                  *(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                   *(int *)(unaff_EBP + -0x20) * 4) + 0x20) =
                       *(int *)(*(int *)(unaff_EBP + -0x68) + 0x16c8) + -1;
                }
                *(int *)(unaff_EBP + -0x5c) = *(int *)(unaff_EBP + -0x5c) + 1;
              }
            }
            else {
              FUN_0043d7d3(*(void **)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                     *(int *)(unaff_EBP + -0x18) * 4),*(HWND *)(unaff_EBP + 8));
            }
          }
          else {
            *(undefined4 *)
             (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                      *(int *)(unaff_EBP + -0x18) * 4) + 0x40) = 1;
            if (**(int **)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                          *(int *)(unaff_EBP + -0x18) * 4) == 2) {
              FUN_0043af87(*(void **)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                     *(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0
                                                               ) + *(int *)(unaff_EBP + -0x18) * 4)
                                             + 0xc) * 4),*(int *)(unaff_EBP + -0x18));
            }
            if (*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                 *(int *)(unaff_EBP + -0x18) * 4) + 0x14) == 2) {
              FUN_0043af87(*(void **)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                     *(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0
                                                               ) + *(int *)(unaff_EBP + -0x18) * 4)
                                             + 0x20) * 4),*(int *)(unaff_EBP + -0x18));
            }
          }
        }
        *(int *)(unaff_EBP + -0x18) = *(int *)(unaff_EBP + -0x18) + 1;
      }
      if (*(int *)(unaff_EBP + -0x54) == 0) {
        uVar1 = 0;
      }
      else {
        *(undefined4 *)(unaff_EBP + -0x1c) = 1;
        while (*(int *)(unaff_EBP + -0x1c) != 0) {
          *(undefined4 *)(unaff_EBP + -0x1c) = 0;
          *(undefined4 *)(unaff_EBP + -0x18) = 0;
          while (*(int *)(unaff_EBP + -0x18) < *(int *)(*(int *)(unaff_EBP + -0x68) + 0x16c8)) {
            if (*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                 *(int *)(unaff_EBP + -0x18) * 4) + 0x40) == 0) {
              if (**(int **)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                            *(int *)(unaff_EBP + -0x18) * 4) == 2) {
                *(undefined4 *)(unaff_EBP + -0x20) =
                     *(undefined4 *)
                      (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                               *(int *)(unaff_EBP + -0x18) * 4) + 0xc);
                *(undefined4 *)(unaff_EBP + -0x50) = 0;
                if (*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                     *(int *)(unaff_EBP + -0x20) * 4) + 0x40) == 0) {
                  *(undefined4 *)(unaff_EBP + -0x5c) = 0;
                  while (*(int *)(unaff_EBP + -0x5c) <
                         *(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                          *(int *)(unaff_EBP + -0x20) * 4) + 0x30)) {
                    if (*(int *)(*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                                  *(int *)(unaff_EBP + -0x20) * 4) + 0x34) + 0xc +
                                *(int *)(unaff_EBP + -0x5c) * 0x14) == *(int *)(unaff_EBP + -0x18))
                    {
                      *(undefined4 *)(unaff_EBP + -0x50) = 1;
                      break;
                    }
                    *(int *)(unaff_EBP + -0x5c) = *(int *)(unaff_EBP + -0x5c) + 1;
                  }
                }
                if (*(int *)(unaff_EBP + -0x50) == 0) {
                  *(undefined4 *)
                   (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                            *(int *)(unaff_EBP + -0x18) * 4) + 0x3c) = 1;
                  **(undefined4 **)
                    (*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                    *(int *)(unaff_EBP + -0x18) * 4) = 0xfffffffd;
                  if (*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                       *(int *)(unaff_EBP + -0x18) * 4) + 0x14) != 1) {
                    *(undefined4 *)
                     (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                              *(int *)(unaff_EBP + -0x18) * 4) + 0x38) = 0xfffffffd;
                  }
                }
              }
              if (*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                   *(int *)(unaff_EBP + -0x18) * 4) + 0x14) == 2) {
                *(undefined4 *)(unaff_EBP + -0x20) =
                     *(undefined4 *)
                      (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                               *(int *)(unaff_EBP + -0x18) * 4) + 0x20);
                *(undefined4 *)(unaff_EBP + -0x50) = 0;
                if (*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                     *(int *)(unaff_EBP + -0x20) * 4) + 0x40) == 0) {
                  *(undefined4 *)(unaff_EBP + -0x5c) = 0;
                  while (*(int *)(unaff_EBP + -0x5c) <
                         *(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                          *(int *)(unaff_EBP + -0x20) * 4) + 0x30)) {
                    if (*(int *)(*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                                  *(int *)(unaff_EBP + -0x20) * 4) + 0x34) + 0xc +
                                *(int *)(unaff_EBP + -0x5c) * 0x14) == *(int *)(unaff_EBP + -0x18))
                    {
                      *(undefined4 *)(unaff_EBP + -0x50) = 1;
                      break;
                    }
                    *(int *)(unaff_EBP + -0x5c) = *(int *)(unaff_EBP + -0x5c) + 1;
                  }
                }
                if (*(int *)(unaff_EBP + -0x50) == 0) {
                  *(undefined4 *)
                   (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                            *(int *)(unaff_EBP + -0x18) * 4) + 0x3c) = 1;
                  *(undefined4 *)
                   (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                            *(int *)(unaff_EBP + -0x18) * 4) + 0x14) = 0xfffffffd;
                  if (**(int **)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                *(int *)(unaff_EBP + -0x18) * 4) != 1) {
                    *(undefined4 *)
                     (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                              *(int *)(unaff_EBP + -0x18) * 4) + 0x38) = 0xfffffffd;
                  }
                }
              }
              if ((**(int **)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                             *(int *)(unaff_EBP + -0x18) * 4) == -3) &&
                 (*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                   *(int *)(unaff_EBP + -0x18) * 4) + 0x14) == -3)) {
                *(undefined4 *)
                 (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                          *(int *)(unaff_EBP + -0x18) * 4) + 0x40) = 1;
                *(undefined4 *)(unaff_EBP + -0x1c) = 1;
                *(int *)(unaff_EBP + -0x54) = *(int *)(unaff_EBP + -0x54) + 1;
              }
            }
            *(int *)(unaff_EBP + -0x18) = *(int *)(unaff_EBP + -0x18) + 1;
          }
        }
        if (*(int *)(unaff_EBP + -0x54) != 0) {
          FUN_004349ff(*(int *)(unaff_EBP + -0x68));
        }
        uVar1 = *(undefined4 *)(unaff_EBP + -0x54);
      }
      ExceptionList = *(void **)(unaff_EBP + -0xc);
      return uVar1;
    }
    if (*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16cc) +
                         *(int *)(unaff_EBP + -0x18) * 4) + 0xd8) != 0) {
      *(undefined4 *)
       (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16cc) + *(int *)(unaff_EBP + -0x18) * 4) +
       0x48) = 1;
      *(undefined4 *)
       (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16cc) + *(int *)(unaff_EBP + -0x18) * 4) +
       0xd8) = 0;
      *(undefined4 *)(unaff_EBP + -0x5c) = 0;
      while (*(int *)(unaff_EBP + -0x5c) <
             *(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16cc) +
                              *(int *)(unaff_EBP + -0x18) * 4) + 0x18)) {
        *(undefined4 *)(unaff_EBP + -0x20) =
             *(undefined4 *)
              (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16cc) +
                       *(int *)(unaff_EBP + -0x18) * 4) + 0xe4 + *(int *)(unaff_EBP + -0x5c) * 4);
        *(undefined4 *)(unaff_EBP + -0x4c) = 0xffffffff;
        if (*(int *)(unaff_EBP + -0x20) != -3) {
          *(undefined4 *)
           (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                    *(int *)(unaff_EBP + -0x20) * 4) + 0x3c) = 1;
          uVar1 = FUN_0043ca9b(*(void **)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                         *(int *)(unaff_EBP + -0x20) * 4),
                               *(int *)(unaff_EBP + -0x18),1,*(int *)(unaff_EBP + -0x5c));
          *(undefined4 *)(unaff_EBP + -0x4c) = uVar1;
          if (((**(int **)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                          *(int *)(unaff_EBP + -0x20) * 4) == 0) &&
              (*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                *(int *)(unaff_EBP + -0x20) * 4) + 4) == *(int *)(unaff_EBP + -0x18)
              )) && (*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                      *(int *)(unaff_EBP + -0x20) * 4) + 8) ==
                     *(int *)(unaff_EBP + -0x5c))) {
            **(undefined4 **)
              (*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) + *(int *)(unaff_EBP + -0x20) * 4) =
                 0xfffffffd;
          }
          else {
            *(undefined4 *)
             (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                      *(int *)(unaff_EBP + -0x20) * 4) + 0x14) = 0xfffffffd;
          }
        }
        if (-1 < *(int *)(unaff_EBP + -0x4c)) {
          FUN_0043cc09(*(void **)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                 *(int *)(unaff_EBP + -0x20) * 4),
                       *(int **)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                *(int *)(unaff_EBP + -0x4c) * 4),*(HWND *)(unaff_EBP + 8));
        }
        *(undefined4 *)
         (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16cc) + *(int *)(unaff_EBP + -0x18) * 4)
          + 0xe4 + *(int *)(unaff_EBP + -0x5c) * 4) = 0xfffffffd;
        if (*(int *)(unaff_EBP + -0x20) != -3) {
          if (((*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                 *(int *)(unaff_EBP + -0x20) * 4) + 0x28) == 2) &&
              (**(int **)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                  *(int *)(unaff_EBP + -0x20) * 4) + 0x2c) ==
               *(int *)(*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                         *(int *)(unaff_EBP + -0x20) * 4) + 0x2c) + 0x14))) &&
             (*(int *)(*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                        *(int *)(unaff_EBP + -0x20) * 4) + 0x2c) + 4) ==
              *(int *)(*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                        *(int *)(unaff_EBP + -0x20) * 4) + 0x2c) + 0x18))) {
            *(undefined4 *)
             (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                      *(int *)(unaff_EBP + -0x20) * 4) + 0x44) = 1;
            *(undefined4 *)
             (*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                               *(int *)(unaff_EBP + -0x20) * 4) + 0x2c) + 8) = 1;
          }
          else if ((*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                     *(int *)(unaff_EBP + -0x20) * 4) + 0x30) == 0) &&
                  (*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                    *(int *)(unaff_EBP + -0x20) * 4) + 0x28) == 2)) {
            *(undefined4 *)(unaff_EBP + -0x48) =
                 **(undefined4 **)
                   (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                            *(int *)(unaff_EBP + -0x20) * 4) + 0x2c);
            *(undefined4 *)(unaff_EBP + -0x14) =
                 *(undefined4 *)
                  (*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                    *(int *)(unaff_EBP + -0x20) * 4) + 0x2c) + 0x14);
            *(undefined4 *)(unaff_EBP + -0x44) =
                 *(undefined4 *)
                  (*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                    *(int *)(unaff_EBP + -0x20) * 4) + 0x2c) + 4);
            *(undefined4 *)(unaff_EBP + -0x58) =
                 *(undefined4 *)
                  (*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                    *(int *)(unaff_EBP + -0x20) * 4) + 0x2c) + 0x18);
            iVar5 = FUN_0043f3b8(*(int *)(unaff_EBP + -0x48) - *(int *)(unaff_EBP + -0x14));
            if ((iVar5 < 10) &&
               (iVar5 = FUN_0043f3b8(*(int *)(unaff_EBP + -0x44) - *(int *)(unaff_EBP + -0x58)),
               iVar5 < 10)) {
              *(undefined4 *)
               (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                        *(int *)(unaff_EBP + -0x20) * 4) + 0x44) = 1;
              *(undefined4 *)
               (*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                 *(int *)(unaff_EBP + -0x20) * 4) + 0x2c) + 8) = 1;
            }
          }
        }
        *(int *)(unaff_EBP + -0x5c) = *(int *)(unaff_EBP + -0x5c) + 1;
      }
      *(undefined4 *)(unaff_EBP + -0x20) =
           *(undefined4 *)
            (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16cc) +
                     *(int *)(unaff_EBP + -0x18) * 4) + 0xe0);
      *(undefined4 *)(unaff_EBP + -0x4c) = 0xffffffff;
      if (*(int *)(unaff_EBP + -0x20) != -3) {
        *(undefined4 *)
         (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) + *(int *)(unaff_EBP + -0x20) * 4)
         + 0x3c) = 1;
        uVar1 = FUN_0043ca9b(*(void **)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                       *(int *)(unaff_EBP + -0x20) * 4),*(int *)(unaff_EBP + -0x18),
                             0,-3);
        *(undefined4 *)(unaff_EBP + -0x4c) = uVar1;
        if ((**(int **)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                       *(int *)(unaff_EBP + -0x20) * 4) == 1) &&
           (*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                             *(int *)(unaff_EBP + -0x20) * 4) + 4) == *(int *)(unaff_EBP + -0x18)))
        {
          **(undefined4 **)
            (*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) + *(int *)(unaff_EBP + -0x20) * 4) =
               0xfffffffd;
        }
        else {
          *(undefined4 *)
           (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                    *(int *)(unaff_EBP + -0x20) * 4) + 0x14) = 0xfffffffd;
        }
      }
      if (-1 < *(int *)(unaff_EBP + -0x4c)) {
        FUN_0043cc09(*(void **)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                               *(int *)(unaff_EBP + -0x20) * 4),
                     *(int **)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                              *(int *)(unaff_EBP + -0x4c) * 4),*(HWND *)(unaff_EBP + 8));
      }
      *(undefined4 *)
       (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16cc) + *(int *)(unaff_EBP + -0x18) * 4) +
       0xe0) = 0xfffffffd;
      if (*(int *)(unaff_EBP + -0x20) != -3) {
        if (((*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                               *(int *)(unaff_EBP + -0x20) * 4) + 0x28) == 2) &&
            (**(int **)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                *(int *)(unaff_EBP + -0x20) * 4) + 0x2c) ==
             *(int *)(*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                       *(int *)(unaff_EBP + -0x20) * 4) + 0x2c) + 0x14))) &&
           (*(int *)(*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                      *(int *)(unaff_EBP + -0x20) * 4) + 0x2c) + 4) ==
            *(int *)(*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                      *(int *)(unaff_EBP + -0x20) * 4) + 0x2c) + 0x18))) {
          *(undefined4 *)
           (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                    *(int *)(unaff_EBP + -0x20) * 4) + 0x44) = 1;
          *(undefined4 *)
           (*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                             *(int *)(unaff_EBP + -0x20) * 4) + 0x2c) + 8) = 1;
        }
        else if ((*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                   *(int *)(unaff_EBP + -0x20) * 4) + 0x30) == 0) &&
                (*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                  *(int *)(unaff_EBP + -0x20) * 4) + 0x28) == 2)) {
          *(undefined4 *)(unaff_EBP + -0x48) =
               **(undefined4 **)
                 (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                          *(int *)(unaff_EBP + -0x20) * 4) + 0x2c);
          *(undefined4 *)(unaff_EBP + -0x14) =
               *(undefined4 *)
                (*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                  *(int *)(unaff_EBP + -0x20) * 4) + 0x2c) + 0x14);
          *(undefined4 *)(unaff_EBP + -0x44) =
               *(undefined4 *)
                (*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                  *(int *)(unaff_EBP + -0x20) * 4) + 0x2c) + 4);
          *(undefined4 *)(unaff_EBP + -0x58) =
               *(undefined4 *)
                (*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                                  *(int *)(unaff_EBP + -0x20) * 4) + 0x2c) + 0x18);
          iVar5 = FUN_0043f3b8(*(int *)(unaff_EBP + -0x48) - *(int *)(unaff_EBP + -0x14));
          if ((iVar5 < 10) &&
             (iVar5 = FUN_0043f3b8(*(int *)(unaff_EBP + -0x44) - *(int *)(unaff_EBP + -0x58)),
             iVar5 < 10)) {
            *(undefined4 *)
             (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                      *(int *)(unaff_EBP + -0x20) * 4) + 0x44) = 1;
            *(undefined4 *)
             (*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16d0) +
                               *(int *)(unaff_EBP + -0x20) * 4) + 0x2c) + 8) = 1;
          }
        }
      }
      *(int *)(unaff_EBP + -0x54) = *(int *)(unaff_EBP + -0x54) + 1;
      if (**(int **)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16cc) +
                    *(int *)(unaff_EBP + -0x18) * 4) == 8) {
        *(undefined4 *)(unaff_EBP + -0x5c) = 0;
        while (*(int *)(unaff_EBP + -0x5c) < *(int *)(*(int *)(unaff_EBP + -0x68) + 0xc4)) {
          iVar5 = _strcmp((char *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16cc) +
                                           *(int *)(unaff_EBP + -0x18) * 4) + 0x50),
                          (char *)(*(int *)(unaff_EBP + -0x68) + 0x160 +
                                  *(int *)(unaff_EBP + -0x5c) * 9));
          if (iVar5 == 0) {
            *(undefined1 *)(*(int *)(unaff_EBP + -0x68) + 0x160 + *(int *)(unaff_EBP + -0x5c) * 9) =
                 0;
            break;
          }
          *(int *)(unaff_EBP + -0x5c) = *(int *)(unaff_EBP + -0x5c) + 1;
        }
        sVar2 = _strlen((char *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16cc) +
                                         *(int *)(unaff_EBP + -0x18) * 4) + 0x50));
        if ((sVar2 == 1) &&
           (iVar5 = _isupper((int)*(char *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16cc)
                                                    + *(int *)(unaff_EBP + -0x18) * 4) + 0x50)),
           iVar5 != 0)) {
          *(undefined4 *)(unaff_EBP + -0x5c) = 0;
          while ((*(int *)(unaff_EBP + -0x5c) < 0x1a &&
                 (*(char *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16cc) +
                                    *(int *)(unaff_EBP + -0x18) * 4) + 0x50) !=
                  *(char *)(unaff_EBP + -0x40 + *(int *)(unaff_EBP + -0x5c))))) {
            *(int *)(unaff_EBP + -0x5c) = *(int *)(unaff_EBP + -0x5c) + 1;
          }
          *(undefined4 *)(*(int *)(unaff_EBP + -0x68) + 0x25ec + *(int *)(unaff_EBP + -0x5c) * 4) =
               0;
        }
      }
      else if (**(int **)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16cc) +
                         *(int *)(unaff_EBP + -0x18) * 4) == 9) {
        *(undefined4 *)(unaff_EBP + -0x5c) = 0;
        while (*(int *)(unaff_EBP + -0x5c) < *(int *)(*(int *)(unaff_EBP + -0x68) + 200)) {
          iVar5 = _strcmp((char *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x68) + 0x16cc) +
                                           *(int *)(unaff_EBP + -0x18) * 4) + 0x50),
                          (char *)(*(int *)(unaff_EBP + -0x68) + 0xd0 +
                                  *(int *)(unaff_EBP + -0x5c) * 9));
          if (iVar5 == 0) {
            *(undefined1 *)(*(int *)(unaff_EBP + -0x68) + 0xd0 + *(int *)(unaff_EBP + -0x5c) * 9) =
                 0;
            break;
          }
          *(int *)(unaff_EBP + -0x5c) = *(int *)(unaff_EBP + -0x5c) + 1;
        }
      }
    }
    *(int *)(unaff_EBP + -0x18) = *(int *)(unaff_EBP + -0x18) + 1;
  } while( true );
}
