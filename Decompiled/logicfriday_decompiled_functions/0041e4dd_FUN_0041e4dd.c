/* 0041e4dd FUN_0041e4dd */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

int FUN_0041e4dd(void)

{
  void *pvVar1;
  undefined4 uVar2;
  int iVar3;
  char *pcVar4;
  long lVar5;
  undefined4 extraout_ECX;
  int unaff_EBP;
  
  FUN_0043f30c();
  *(uint *)(unaff_EBP + -0x14) = DAT_00451a00 ^ *(uint *)(unaff_EBP + 4);
  *(undefined4 *)(unaff_EBP + -0x19c) = extraout_ECX;
  *(undefined4 *)(unaff_EBP + -0x11c) = 0;
  *(undefined4 *)(unaff_EBP + -0x138) = 0;
  *(undefined4 *)(unaff_EBP + -0x148) = 0;
  *(undefined4 *)(unaff_EBP + -0x150) = 0;
  *(undefined4 *)(unaff_EBP + -0x130) = 0x800;
  *(undefined4 *)(unaff_EBP + -0x13c) = 1;
  pvVar1 = _malloc(*(size_t *)(unaff_EBP + -0x130));
  *(void **)(unaff_EBP + -0x11c) = pvVar1;
  *(undefined4 *)(*(int *)(unaff_EBP + -0x19c) + 0x1650) = 0;
  uVar2 = FUN_0043e6f2(*(char **)(unaff_EBP + 0xc),"rt");
  *(undefined4 *)(*(int *)(unaff_EBP + -0x19c) + 0x16d4) = uVar2;
  if (*(int *)(*(int *)(unaff_EBP + -0x19c) + 0x16d4) == 0) {
    iVar3 = 0x2f0010;
  }
  else {
    pcVar4 = FUN_0043f99d(*(char **)(unaff_EBP + -0x11c),*(int *)(unaff_EBP + -0x130),
                          *(FILE **)(*(int *)(unaff_EBP + -0x19c) + 0x16d4));
    if (pcVar4 == (char *)0x0) {
      iVar3 = 0x1a0000;
    }
    else {
      do {
        pcVar4 = _strstr(*(char **)(unaff_EBP + -0x11c),"does not fanout");
        if (pcVar4 == (char *)0x0) {
          pcVar4 = _strstr(*(char **)(unaff_EBP + -0x11c),"nodes=");
          *(char **)(unaff_EBP + -0x10) = pcVar4;
          if (*(int *)(unaff_EBP + -0x10) != 0) goto LAB_0041e6f8;
          pcVar4 = _strstr(*(char **)(unaff_EBP + -0x11c),"already mapped");
          if (pcVar4 == (char *)0x0) goto LAB_0041e69d;
          _fclose(*(FILE **)(*(int *)(unaff_EBP + -0x19c) + 0x16d4));
          if (DAT_00452eb8 == 0) {
            MessageBoxA(*(HWND *)(*(int *)(unaff_EBP + -0x19c) + 0x16f0),
                        "No gates are required to implement the function.","Map to Gates",0);
          }
          if (DAT_00452e8c != 0) {
            FID_conflict__fwprintf(DAT_004528a8,(wchar_t *)"%d\t%d\n",0,0);
          }
          iVar3 = 0;
          goto LAB_00420009;
        }
        pcVar4 = FUN_0043f99d(*(char **)(unaff_EBP + -0x11c),*(int *)(unaff_EBP + -0x130),
                              *(FILE **)(*(int *)(unaff_EBP + -0x19c) + 0x16d4));
      } while (pcVar4 != (char *)0x0);
      _fclose(*(FILE **)(*(int *)(unaff_EBP + -0x19c) + 0x16d4));
      iVar3 = 0x1a0000;
    }
  }
  goto LAB_00420009;
  while( true ) {
    pcVar4 = _strstr(*(char **)(unaff_EBP + -0x11c),"nodes=");
    *(char **)(unaff_EBP + -0x10) = pcVar4;
    if (*(int *)(unaff_EBP + -0x10) != 0) break;
LAB_0041e69d:
    pcVar4 = FUN_0043f99d(*(char **)(unaff_EBP + -0x11c),*(int *)(unaff_EBP + -0x130),
                          *(FILE **)(*(int *)(unaff_EBP + -0x19c) + 0x16d4));
    if (pcVar4 == (char *)0x0) {
      _fclose(*(FILE **)(*(int *)(unaff_EBP + -0x19c) + 0x16d4));
      iVar3 = 0x1a0000;
      goto LAB_00420009;
    }
  }
LAB_0041e6f8:
  pcVar4 = _strchr(*(char **)(unaff_EBP + -0x10),0x3d);
  *(char **)(unaff_EBP + -300) = pcVar4;
  *(int *)(unaff_EBP + -300) = *(int *)(unaff_EBP + -300) + 1;
  lVar5 = _atol(*(char **)(unaff_EBP + -300));
  *(long *)(unaff_EBP + -0x150) = lVar5;
  if (*(int *)(unaff_EBP + -0x150) == 0) {
    _fclose(*(FILE **)(*(int *)(unaff_EBP + -0x19c) + 0x16d4));
    iVar3 = 0x1a0000;
  }
  else {
    *(int *)(unaff_EBP + -0x134) = *(int *)(unaff_EBP + -0x150) * 0x50;
    if ((uint)(*(int *)(*(int *)(unaff_EBP + -0x19c) + 0x1664) * 0x7fff) <
        *(uint *)(unaff_EBP + -0x134)) {
      while ((uint)(*(int *)(*(int *)(unaff_EBP + -0x19c) + 0x1664) * 0x7fff) <
             *(uint *)(unaff_EBP + -0x134)) {
        *(int *)(*(int *)(unaff_EBP + -0x19c) + 0x1664) =
             *(int *)(*(int *)(unaff_EBP + -0x19c) + 0x1664) + 1;
      }
      pvVar1 = _realloc(*(void **)(*(int *)(unaff_EBP + -0x19c) + 0x274),
                        *(int *)(*(int *)(unaff_EBP + -0x19c) + 0x1664) * 0x7fff);
      *(void **)(*(int *)(unaff_EBP + -0x19c) + 0x274) = pvVar1;
    }
    while ((pcVar4 = FUN_0043f99d(*(char **)(unaff_EBP + -0x11c),*(int *)(unaff_EBP + -0x130),
                                  *(FILE **)(*(int *)(unaff_EBP + -0x19c) + 0x16d4)),
           pcVar4 != (char *)0x0 &&
           ((**(char **)(unaff_EBP + -0x11c) == '[' || (**(char **)(unaff_EBP + -0x11c) == '{')))))
    {
      *(int *)(*(int *)(unaff_EBP + -0x19c) + 0x1650) =
           *(int *)(*(int *)(unaff_EBP + -0x19c) + 0x1650) + 1;
    }
    if (*(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4) == 0) {
      *(undefined4 *)(*(int *)(unaff_EBP + -0x19c) + 0x260) = 1;
    }
    else {
      *(undefined4 *)(unaff_EBP + -0x18c) = *(undefined4 *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4);
      *(undefined4 *)(unaff_EBP + -0x188) = *(undefined4 *)(unaff_EBP + -0x18c);
      if (*(int *)(unaff_EBP + -0x188) == 0) {
        *(undefined4 *)(unaff_EBP + -0x1a0) = 0;
      }
      else {
        pvVar1 = FUN_0041338b(*(void **)(unaff_EBP + -0x188),3);
        *(void **)(unaff_EBP + -0x1a0) = pvVar1;
      }
      *(undefined4 *)(*(int *)(unaff_EBP + -0x19c) + 0x260) = 0;
    }
    *(undefined4 *)(*(int *)(unaff_EBP + -0x19c) + 0x1654) =
         *(undefined4 *)(*(int *)(unaff_EBP + -0x19c) + 0xc4);
    *(int *)(*(int *)(unaff_EBP + -0x19c) + 0x1658) =
         *(int *)(*(int *)(unaff_EBP + -0x19c) + 0x1654) +
         *(int *)(*(int *)(unaff_EBP + -0x19c) + 0x1650);
    *(int *)(*(int *)(unaff_EBP + -0x19c) + 0x1650) =
         *(int *)(*(int *)(unaff_EBP + -0x19c) + 0xc4) +
         *(int *)(*(int *)(unaff_EBP + -0x19c) + 200) +
         *(int *)(*(int *)(unaff_EBP + -0x19c) + 0x1650);
    *(undefined4 *)(unaff_EBP + -400) = *(undefined4 *)(*(int *)(unaff_EBP + -0x19c) + 0x1650);
    pvVar1 = operator_new(*(int *)(unaff_EBP + -400) * 0xfc + 4);
    *(void **)(unaff_EBP + -0x198) = pvVar1;
    *(undefined4 *)(unaff_EBP + -4) = 0;
    if (*(int *)(unaff_EBP + -0x198) == 0) {
      *(undefined4 *)(unaff_EBP + -0x1a4) = 0;
    }
    else {
      **(undefined4 **)(unaff_EBP + -0x198) = *(undefined4 *)(unaff_EBP + -400);
      _eh_vector_constructor_iterator_
                ((void *)(*(int *)(unaff_EBP + -0x198) + 4),0xfc,*(int *)(unaff_EBP + -400),
                 FUN_004175df,FUN_0043961a);
      *(int *)(unaff_EBP + -0x1a4) = *(int *)(unaff_EBP + -0x198) + 4;
    }
    *(undefined4 *)(unaff_EBP + -0x194) = *(undefined4 *)(unaff_EBP + -0x1a4);
    *(undefined4 *)(unaff_EBP + -4) = 0xffffffff;
    *(undefined4 *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4) = *(undefined4 *)(unaff_EBP + -0x194);
    *(undefined4 *)(unaff_EBP + -0x124) = 0;
    while (*(int *)(unaff_EBP + -0x124) < *(int *)(*(int *)(unaff_EBP + -0x19c) + 0x1654)) {
      *(undefined4 *)
       (*(int *)(unaff_EBP + -0x124) * 0xfc + *(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4)) = 8;
      FUN_0043ebd0((uint *)(*(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4) + 0x50 +
                           *(int *)(unaff_EBP + -0x124) * 0xfc),
                   (uint *)(*(int *)(unaff_EBP + -0x19c) + 0x160 + *(int *)(unaff_EBP + -0x124) * 9)
                  );
      *(undefined4 *)
       (*(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4) + 0x18 + *(int *)(unaff_EBP + -0x124) * 0xfc)
           = 0;
      *(undefined4 *)
       (*(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4) + 0x3c + *(int *)(unaff_EBP + -0x124) * 0xfc)
           = 0;
      *(undefined4 *)
       (*(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4) + 0x40 + *(int *)(unaff_EBP + -0x124) * 0xfc)
           = *(undefined4 *)(unaff_EBP + -0x124);
      *(undefined4 *)
       (*(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4) + 0xb4 + *(int *)(unaff_EBP + -0x124) * 0xfc)
           = 1;
      *(undefined4 *)
       (*(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4) + 0x48 + *(int *)(unaff_EBP + -0x124) * 0xfc)
           = 0;
      *(undefined4 *)
       (*(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4) + 0x44 + *(int *)(unaff_EBP + -0x124) * 0xfc)
           = *(undefined4 *)(unaff_EBP + -0x124);
      *(int *)(unaff_EBP + -0x124) = *(int *)(unaff_EBP + -0x124) + 1;
    }
    *(undefined4 *)(unaff_EBP + -0x124) = *(undefined4 *)(*(int *)(unaff_EBP + -0x19c) + 0x1658);
    while (*(int *)(unaff_EBP + -0x124) < *(int *)(*(int *)(unaff_EBP + -0x19c) + 0x1650)) {
      *(undefined4 *)
       (*(int *)(unaff_EBP + -0x124) * 0xfc + *(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4)) = 9;
      FUN_0043ebd0((uint *)(*(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4) + 0x50 +
                           *(int *)(unaff_EBP + -0x124) * 0xfc),
                   (uint *)(*(int *)(unaff_EBP + -0x19c) + 0xd0 +
                           (*(int *)(unaff_EBP + -0x124) -
                           *(int *)(*(int *)(unaff_EBP + -0x19c) + 0x1658)) * 9));
      FUN_0043ebd0((uint *)(*(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4) + 4 +
                           *(int *)(unaff_EBP + -0x124) * 0xfc),
                   (uint *)(*(int *)(unaff_EBP + -0x19c) + 0xd0 +
                           (*(int *)(unaff_EBP + -0x124) -
                           *(int *)(*(int *)(unaff_EBP + -0x19c) + 0x1658)) * 9));
      *(undefined4 *)
       (*(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4) + 0x18 + *(int *)(unaff_EBP + -0x124) * 0xfc)
           = 1;
      *(undefined4 *)
       (*(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4) + 0x3c + *(int *)(unaff_EBP + -0x124) * 0xfc)
           = 0;
      *(undefined4 *)
       (*(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4) + 0xb4 + *(int *)(unaff_EBP + -0x124) * 0xfc)
           = 0;
      *(undefined4 *)
       (*(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4) + 0x48 + *(int *)(unaff_EBP + -0x124) * 0xfc)
           = 0;
      *(undefined4 *)
       (*(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4) + 0x1c + *(int *)(unaff_EBP + -0x124) * 0xfc)
           = 0xffffffff;
      FUN_0041770d((int *)(*(int *)(unaff_EBP + -0x124) * 0xfc +
                          *(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4)));
      *(undefined4 *)
       (*(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4) + 0x44 + *(int *)(unaff_EBP + -0x124) * 0xfc)
           = *(undefined4 *)(unaff_EBP + -0x124);
      *(int *)(unaff_EBP + -0x124) = *(int *)(unaff_EBP + -0x124) + 1;
    }
    _rewind(*(FILE **)(*(int *)(unaff_EBP + -0x19c) + 0x16d4));
    do {
      FUN_0043f99d(*(char **)(unaff_EBP + -0x11c),*(int *)(unaff_EBP + -0x130),
                   *(FILE **)(*(int *)(unaff_EBP + -0x19c) + 0x16d4));
      pcVar4 = _strstr(*(char **)(unaff_EBP + -0x11c),"nodes=");
      *(char **)(unaff_EBP + -0x10) = pcVar4;
    } while (*(int *)(unaff_EBP + -0x10) == 0);
    *(undefined4 *)(unaff_EBP + -0x124) = *(undefined4 *)(*(int *)(unaff_EBP + -0x19c) + 0x1654);
    while (*(int *)(unaff_EBP + -0x124) < *(int *)(*(int *)(unaff_EBP + -0x19c) + 0x1658)) {
      pcVar4 = FUN_0043f99d(*(char **)(unaff_EBP + -0x11c),*(int *)(unaff_EBP + -0x130),
                            *(FILE **)(*(int *)(unaff_EBP + -0x19c) + 0x16d4));
      if (pcVar4 == (char *)0x0) {
        _fclose(*(FILE **)(*(int *)(unaff_EBP + -0x19c) + 0x16d4));
        iVar3 = 0x1a0000;
        goto LAB_00420009;
      }
      if (**(char **)(unaff_EBP + -0x11c) == '{') {
        *(undefined4 *)(unaff_EBP + -0x13c) = 1;
        *(int *)(unaff_EBP + -0x10) = *(int *)(unaff_EBP + -0x11c) + 1;
        *(undefined4 *)(unaff_EBP + -300) = *(undefined4 *)(unaff_EBP + -0x10);
        do {
          *(int *)(unaff_EBP + -300) = *(int *)(unaff_EBP + -300) + 1;
          if (**(char **)(unaff_EBP + -300) == '\0') {
            _fclose(*(FILE **)(*(int *)(unaff_EBP + -0x19c) + 0x16d4));
            iVar3 = 0x1a0000;
            goto LAB_00420009;
          }
          if ((**(char **)(unaff_EBP + -300) == ',') || (**(char **)(unaff_EBP + -300) == '}')) {
            _strncpy((char *)(unaff_EBP + -0x114),*(char **)(unaff_EBP + -0x10),
                     *(int *)(unaff_EBP + -300) - *(int *)(unaff_EBP + -0x10));
            *(undefined1 *)
             (unaff_EBP + -0x114 + (*(int *)(unaff_EBP + -300) - *(int *)(unaff_EBP + -0x10))) = 0;
            if (*(int *)(unaff_EBP + -0x13c) != 0) {
              FUN_0043ebd0((uint *)(*(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4) + 4 +
                                   *(int *)(unaff_EBP + -0x124) * 0xfc),(uint *)(unaff_EBP + -0x114)
                          );
              *(undefined4 *)(unaff_EBP + -0x13c) = 0;
            }
            *(undefined4 *)(unaff_EBP + -0x14c) =
                 *(undefined4 *)(*(int *)(unaff_EBP + -0x19c) + 0x1658);
            while (*(int *)(unaff_EBP + -0x14c) < *(int *)(*(int *)(unaff_EBP + -0x19c) + 0x1650)) {
              iVar3 = _strcmp((char *)(unaff_EBP + -0x114),
                              (char *)(*(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4) + 4 +
                                      *(int *)(unaff_EBP + -0x14c) * 0xfc));
              if (iVar3 == 0) {
                *(undefined4 *)
                 (*(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4) + 0x1c +
                 *(int *)(unaff_EBP + -0x14c) * 0xfc) = *(undefined4 *)(unaff_EBP + -0x124);
                *(undefined4 *)(unaff_EBP + -0x120) = *(undefined4 *)(unaff_EBP + -0x14c);
              }
              *(int *)(unaff_EBP + -0x14c) = *(int *)(unaff_EBP + -0x14c) + 1;
            }
            if (**(char **)(unaff_EBP + -300) == ',') {
              *(int *)(unaff_EBP + -0x10) = *(int *)(unaff_EBP + -300) + 1;
              *(undefined4 *)(unaff_EBP + -300) = *(undefined4 *)(unaff_EBP + -0x10);
            }
          }
        } while (**(char **)(unaff_EBP + -300) != '}');
        *(undefined4 *)(unaff_EBP + -0x10) = *(undefined4 *)(unaff_EBP + -300);
      }
      else {
        pcVar4 = _strchr(*(char **)(unaff_EBP + -0x11c),0x5d);
        *(char **)(unaff_EBP + -0x10) = pcVar4;
        if (*(int *)(unaff_EBP + -0x10) == 0) {
          _fclose(*(FILE **)(*(int *)(unaff_EBP + -0x19c) + 0x16d4));
          iVar3 = 0x1a0000;
          goto LAB_00420009;
        }
        _strncpy((char *)(*(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4) + 4 +
                         *(int *)(unaff_EBP + -0x124) * 0xfc),*(char **)(unaff_EBP + -0x11c),
                 (*(int *)(unaff_EBP + -0x10) - *(int *)(unaff_EBP + -0x11c)) + 1);
        *(undefined1 *)
         (*(int *)(unaff_EBP + -0x124) * 0xfc + *(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4) + 5 +
         (*(int *)(unaff_EBP + -0x10) - *(int *)(unaff_EBP + -0x11c))) = 0;
        _strncpy((char *)(*(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4) + 0x50 +
                         *(int *)(unaff_EBP + -0x124) * 0xfc),*(char **)(unaff_EBP + -0x11c),
                 (*(int *)(unaff_EBP + -0x10) - *(int *)(unaff_EBP + -0x11c)) + 1);
        *(undefined1 *)
         (*(int *)(unaff_EBP + -0x124) * 0xfc + *(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4) +
          0x51 + (*(int *)(unaff_EBP + -0x10) - *(int *)(unaff_EBP + -0x11c))) = 0;
      }
      while (iVar3 = _isalnum((int)**(char **)(unaff_EBP + -0x10)), iVar3 == 0) {
        *(int *)(unaff_EBP + -0x10) = *(int *)(unaff_EBP + -0x10) + 1;
      }
      iVar3 = __strnicmp(*(char **)(unaff_EBP + -0x10),"inv",3);
      if (iVar3 == 0) {
        *(undefined4 *)
         (*(int *)(unaff_EBP + -0x124) * 0xfc + *(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4)) = 0;
      }
      else {
        iVar3 = __strnicmp(*(char **)(unaff_EBP + -0x10),"nan",3);
        if (iVar3 == 0) {
          *(undefined4 *)
           (*(int *)(unaff_EBP + -0x124) * 0xfc + *(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4)) =
               1;
        }
        else {
          iVar3 = __strnicmp(*(char **)(unaff_EBP + -0x10),"nor",3);
          if (iVar3 == 0) {
            *(undefined4 *)
             (*(int *)(unaff_EBP + -0x124) * 0xfc + *(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4))
                 = 2;
          }
          else {
            iVar3 = __strnicmp(*(char **)(unaff_EBP + -0x10),"exo",3);
            if (iVar3 == 0) {
              *(undefined4 *)
               (*(int *)(unaff_EBP + -0x124) * 0xfc + *(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4)
               ) = 3;
            }
            else {
              iVar3 = __strnicmp(*(char **)(unaff_EBP + -0x10),"exn",3);
              if (iVar3 == 0) {
                *(undefined4 *)
                 (*(int *)(unaff_EBP + -0x124) * 0xfc +
                 *(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4)) = 4;
              }
              else {
                iVar3 = __strnicmp(*(char **)(unaff_EBP + -0x10),"mux",3);
                if (iVar3 == 0) {
                  *(undefined4 *)
                   (*(int *)(unaff_EBP + -0x124) * 0xfc +
                   *(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4)) = 5;
                }
                else {
                  iVar3 = __strnicmp(*(char **)(unaff_EBP + -0x10),"and",3);
                  if (iVar3 == 0) {
                    *(undefined4 *)
                     (*(int *)(unaff_EBP + -0x124) * 0xfc +
                     *(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4)) = 6;
                  }
                  else {
                    iVar3 = __strnicmp(*(char **)(unaff_EBP + -0x10),"or",2);
                    if (iVar3 == 0) {
                      *(undefined4 *)
                       (*(int *)(unaff_EBP + -0x124) * 0xfc +
                       *(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4)) = 7;
                    }
                    else {
                      iVar3 = __strnicmp(*(char **)(unaff_EBP + -0x10),"one",3);
                      if (iVar3 == 0) {
                        *(undefined4 *)
                         (*(int *)(unaff_EBP + -0x124) * 0xfc +
                         *(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4)) = 10;
                      }
                      else {
                        iVar3 = __strnicmp(*(char **)(unaff_EBP + -0x10),"zer",3);
                        if (iVar3 == 0) {
                          *(undefined4 *)
                           (*(int *)(unaff_EBP + -0x124) * 0xfc +
                           *(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4)) = 0xb;
                        }
                      }
                    }
                  }
                }
              }
            }
          }
        }
      }
      while (iVar3 = _isdigit((int)**(char **)(unaff_EBP + -0x10)), iVar3 == 0) {
        *(int *)(unaff_EBP + -0x10) = *(int *)(unaff_EBP + -0x10) + 1;
      }
      FUN_0043ed39((char *)(unaff_EBP + -0x114),&DAT_0044c980);
      lVar5 = _atol((char *)(unaff_EBP + -0x114));
      *(long *)(*(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4) + 0x18 +
               *(int *)(unaff_EBP + -0x124) * 0xfc) = lVar5;
      if (*(int *)(*(int *)(unaff_EBP + -0x124) * 0xfc +
                  *(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4)) == 5) {
        *(undefined4 *)
         (*(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4) + 0x18 +
         *(int *)(unaff_EBP + -0x124) * 0xfc) = 3;
      }
      *(undefined4 *)(unaff_EBP + -0x154) = 0;
      while (*(int *)(unaff_EBP + -0x154) <
             *(int *)(*(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4) + 0x18 +
                     *(int *)(unaff_EBP + -0x124) * 0xfc)) {
        while (**(char **)(unaff_EBP + -0x10) != '=') {
          *(int *)(unaff_EBP + -0x10) = *(int *)(unaff_EBP + -0x10) + 1;
        }
        *(int *)(unaff_EBP + -0x10) = *(int *)(unaff_EBP + -0x10) + 1;
        if (**(char **)(unaff_EBP + -0x10) == '[') {
          *(int *)(unaff_EBP + -0x10) = *(int *)(unaff_EBP + -0x10) + 1;
          lVar5 = _atol(*(char **)(unaff_EBP + -0x10));
          *(long *)(*(int *)(unaff_EBP + -0x124) * 0xfc +
                    *(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4) + 0x1c +
                   *(int *)(unaff_EBP + -0x154) * 4) =
               lVar5 + *(int *)(*(int *)(unaff_EBP + -0x19c) + 0x1654);
        }
        else if (**(char **)(unaff_EBP + -0x10) == '{') {
          *(int *)(unaff_EBP + -0x10) = *(int *)(unaff_EBP + -0x10) + 1;
          *(undefined4 *)(unaff_EBP + -300) = *(undefined4 *)(unaff_EBP + -0x10);
          do {
            *(int *)(unaff_EBP + -300) = *(int *)(unaff_EBP + -300) + 1;
            if (**(char **)(unaff_EBP + -300) == '\0') {
              _fclose(*(FILE **)(*(int *)(unaff_EBP + -0x19c) + 0x16d4));
              iVar3 = 0x1a0000;
              goto LAB_00420009;
            }
            if ((**(char **)(unaff_EBP + -300) == ',') || (**(char **)(unaff_EBP + -300) == '}')) {
              _strncpy((char *)(unaff_EBP + -0x114),*(char **)(unaff_EBP + -0x10),
                       *(int *)(unaff_EBP + -300) - *(int *)(unaff_EBP + -0x10));
              *(undefined1 *)
               (unaff_EBP + -0x114 + (*(int *)(unaff_EBP + -300) - *(int *)(unaff_EBP + -0x10))) = 0
              ;
              *(undefined4 *)(unaff_EBP + -0x140) =
                   *(undefined4 *)(*(int *)(unaff_EBP + -0x19c) + 0x1658);
              while (*(int *)(unaff_EBP + -0x140) < *(int *)(*(int *)(unaff_EBP + -0x19c) + 0x1650))
              {
                iVar3 = _strcmp((char *)(unaff_EBP + -0x114),
                                (char *)(*(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4) + 4 +
                                        *(int *)(unaff_EBP + -0x140) * 0xfc));
                if (iVar3 == 0) {
                  *(undefined4 *)
                   (*(int *)(unaff_EBP + -0x124) * 0xfc +
                    *(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4) + 0x1c +
                   *(int *)(unaff_EBP + -0x154) * 4) =
                       *(undefined4 *)
                        (*(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4) + 0x1c +
                        *(int *)(unaff_EBP + -0x140) * 0xfc);
                }
                *(int *)(unaff_EBP + -0x140) = *(int *)(unaff_EBP + -0x140) + 1;
              }
            }
          } while ((**(char **)(unaff_EBP + -300) != '}') && (**(char **)(unaff_EBP + -300) != ','))
          ;
        }
        else {
          *(undefined4 *)(unaff_EBP + -0x140) = 0;
          while ((**(char **)(unaff_EBP + -0x10) != ' ' && (**(char **)(unaff_EBP + -0x10) != ')')))
          {
            *(undefined1 *)(unaff_EBP + -0x114 + *(int *)(unaff_EBP + -0x140)) =
                 **(undefined1 **)(unaff_EBP + -0x10);
            *(int *)(unaff_EBP + -0x140) = *(int *)(unaff_EBP + -0x140) + 1;
            *(int *)(unaff_EBP + -0x10) = *(int *)(unaff_EBP + -0x10) + 1;
          }
          *(undefined1 *)(unaff_EBP + -0x114 + *(int *)(unaff_EBP + -0x140)) = 0;
          *(undefined4 *)(unaff_EBP + -0x158) = 0;
          *(undefined4 *)(unaff_EBP + -0x140) = 0;
          while (*(int *)(unaff_EBP + -0x140) < *(int *)(*(int *)(unaff_EBP + -0x19c) + 0xc4)) {
            iVar3 = _strcmp((char *)(*(int *)(unaff_EBP + -0x19c) + 0x160 +
                                    *(int *)(unaff_EBP + -0x140) * 9),(char *)(unaff_EBP + -0x114));
            if (iVar3 == 0) {
              *(undefined4 *)
               (*(int *)(unaff_EBP + -0x124) * 0xfc + *(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4)
                + 0x1c + *(int *)(unaff_EBP + -0x154) * 4) = *(undefined4 *)(unaff_EBP + -0x140);
              *(undefined4 *)(unaff_EBP + -0x158) = 1;
              break;
            }
            *(int *)(unaff_EBP + -0x140) = *(int *)(unaff_EBP + -0x140) + 1;
          }
          if (*(int *)(unaff_EBP + -0x158) == 0) {
            _fclose(*(FILE **)(*(int *)(unaff_EBP + -0x19c) + 0x16d4));
            iVar3 = 0x1a0000;
            goto LAB_00420009;
          }
        }
        *(int *)(unaff_EBP + -0x154) = *(int *)(unaff_EBP + -0x154) + 1;
      }
      FUN_0041770d((int *)(*(int *)(unaff_EBP + -0x124) * 0xfc +
                          *(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4)));
      *(undefined4 *)
       (*(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4) + 0x44 + *(int *)(unaff_EBP + -0x124) * 0xfc)
           = *(undefined4 *)(unaff_EBP + -0x124);
      *(undefined4 *)
       (*(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4) + 0x48 + *(int *)(unaff_EBP + -0x124) * 0xfc)
           = 0;
      *(int *)(unaff_EBP + -0x124) = *(int *)(unaff_EBP + -0x124) + 1;
    }
    if (*(int *)(unaff_EBP + -0x138) != *(int *)(*(int *)(unaff_EBP + -0x19c) + 200)) {
      *(undefined4 *)(unaff_EBP + -0x124) = 0;
      while (*(int *)(unaff_EBP + -0x124) < *(int *)(*(int *)(unaff_EBP + -0x19c) + 200)) {
        if (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4) + 0x1c +
                    (*(int *)(*(int *)(unaff_EBP + -0x19c) + 0x1658) + *(int *)(unaff_EBP + -0x124))
                    * 0xfc) == -1) {
          *(int *)(unaff_EBP + -0x160) = *(int *)(*(int *)(unaff_EBP + -0x19c) + 0xc4) + -1;
          *(undefined4 *)(unaff_EBP + -0x164) = 0;
          *(undefined4 *)(unaff_EBP + -0x14c) = 0;
          while (*(int *)(unaff_EBP + -0x14c) < *(int *)(*(int *)(unaff_EBP + -0x19c) + 500)) {
            if (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x19c) + 0x1fc +
                                 *(int *)(unaff_EBP + -0x124) * 4) +
                        *(int *)(unaff_EBP + -0x14c) * 4) != 0) {
              *(undefined4 *)(unaff_EBP + -0x15c) = 0xffffffff;
              *(undefined4 *)(unaff_EBP + -0x168) = *(undefined4 *)(unaff_EBP + -0x160);
              while (-1 < *(int *)(unaff_EBP + -0x168)) {
                if ((*(uint *)(*(int *)(*(int *)(unaff_EBP + -0x19c) + 0x1f8) + 8 +
                              *(int *)(unaff_EBP + -0x14c) * 0xc) &
                    1 << ((byte)*(undefined4 *)(unaff_EBP + -0x168) & 0x1f)) == 0) {
                  if (*(int *)(unaff_EBP + -0x15c) != -1) {
                    *(undefined4 *)(unaff_EBP + -0x15c) = 0xffffffff;
                    break;
                  }
                  *(undefined4 *)(unaff_EBP + -0x15c) = *(undefined4 *)(unaff_EBP + -0x168);
                }
                *(int *)(unaff_EBP + -0x168) = *(int *)(unaff_EBP + -0x168) + -1;
              }
              if ((*(int *)(unaff_EBP + -0x15c) != -1) &&
                 ((*(uint *)(*(int *)(*(int *)(unaff_EBP + -0x19c) + 0x1f8) + 4 +
                            *(int *)(unaff_EBP + -0x14c) * 0xc) &
                  1 << ((byte)*(undefined4 *)(unaff_EBP + -0x15c) & 0x1f)) != 0)) {
                *(int *)(*(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4) + 0x1c +
                        (*(int *)(*(int *)(unaff_EBP + -0x19c) + 0x1658) +
                        *(int *)(unaff_EBP + -0x124)) * 0xfc) =
                     (*(int *)(*(int *)(unaff_EBP + -0x19c) + 0xc4) - *(int *)(unaff_EBP + -0x15c))
                     + -1;
                *(undefined4 *)(unaff_EBP + -0x164) = 1;
                break;
              }
            }
            *(int *)(unaff_EBP + -0x14c) = *(int *)(unaff_EBP + -0x14c) + 1;
          }
          if (*(int *)(unaff_EBP + -0x164) == 0) {
            iVar3 = 0x1a0000;
            goto LAB_00420009;
          }
        }
        *(int *)(unaff_EBP + -0x124) = *(int *)(unaff_EBP + -0x124) + 1;
      }
    }
    *(undefined4 *)(unaff_EBP + -0x148) = 0;
    do {
      do {
        pcVar4 = FUN_0043f99d(*(char **)(unaff_EBP + -0x11c),*(int *)(unaff_EBP + -0x130),
                              *(FILE **)(*(int *)(unaff_EBP + -0x19c) + 0x16d4));
        if (pcVar4 == (char *)0x0) goto LAB_0041f90a;
        pcVar4 = _strstr(*(char **)(unaff_EBP + -0x11c),"Total number");
      } while (pcVar4 == (char *)0x0);
      pcVar4 = _strchr(*(char **)(unaff_EBP + -0x11c),0x3d);
      *(char **)(unaff_EBP + -0x10) = pcVar4;
    } while (*(int *)(unaff_EBP + -0x10) == 0);
    lVar5 = _atol((char *)(*(int *)(unaff_EBP + -0x10) + 1));
    *(long *)(unaff_EBP + -0x148) = lVar5 + 2;
LAB_0041f90a:
    if (0x40 < *(uint *)(unaff_EBP + -0x148)) {
      _free(*(void **)(*(int *)(unaff_EBP + -0x19c) + 0x1668));
      pvVar1 = _malloc(*(int *)(unaff_EBP + -0x148) << 2);
      *(void **)(*(int *)(unaff_EBP + -0x19c) + 0x1668) = pvVar1;
    }
    if (*(int *)(unaff_EBP + -0x148) != 0) {
      if (*(uint *)(unaff_EBP + -0x130) >> 3 < *(uint *)(*(int *)(unaff_EBP + -0x19c) + 0x1650)) {
        *(int *)(unaff_EBP + -0x130) = *(int *)(*(int *)(unaff_EBP + -0x19c) + 0x1650) << 3;
        _free(*(void **)(unaff_EBP + -0x11c));
        pvVar1 = _malloc(*(size_t *)(unaff_EBP + -0x130));
        *(void **)(unaff_EBP + -0x11c) = pvVar1;
      }
      *(undefined4 *)(unaff_EBP + -0x16c) = *(undefined4 *)(*(int *)(unaff_EBP + -0x19c) + 0xc4);
      if (*(uint *)(*(int *)(unaff_EBP + -0x19c) + 0xc4) <
          *(uint *)(*(int *)(unaff_EBP + -0x19c) + 200)) {
        *(undefined4 *)(unaff_EBP + -0x16c) = *(undefined4 *)(*(int *)(unaff_EBP + -0x19c) + 200);
      }
      **(undefined4 **)(*(int *)(unaff_EBP + -0x19c) + 0x1668) =
           *(undefined4 *)(*(int *)(unaff_EBP + -0x19c) + 0xc4);
      *(undefined4 *)(unaff_EBP + -0x124) = 0;
      while (*(int *)(unaff_EBP + -0x124) < *(int *)(unaff_EBP + -0x148) + -1) {
        pcVar4 = FUN_0043f99d(*(char **)(unaff_EBP + -0x11c),*(int *)(unaff_EBP + -0x130),
                              *(FILE **)(*(int *)(unaff_EBP + -0x19c) + 0x16d4));
        if ((pcVar4 != (char *)0x0) && (*(int *)(unaff_EBP + -0x124) != 0)) {
          iVar3 = FUN_00420023(*(void **)(unaff_EBP + -0x19c),*(char **)(unaff_EBP + -0x11c),
                               *(int *)(unaff_EBP + -0x124));
          *(int *)(unaff_EBP + -0x170) = iVar3;
          if (*(int *)(unaff_EBP + -0x16c) < *(int *)(unaff_EBP + -0x170)) {
            *(undefined4 *)(unaff_EBP + -0x16c) = *(undefined4 *)(unaff_EBP + -0x170);
          }
        }
        *(int *)(unaff_EBP + -0x124) = *(int *)(unaff_EBP + -0x124) + 1;
      }
      *(undefined4 *)(*(int *)(unaff_EBP + -0x19c) + 0x2670) = *(undefined4 *)(unaff_EBP + -0x16c);
      *(undefined4 *)(*(int *)(unaff_EBP + -0x19c) + 0x2674) = *(undefined4 *)(unaff_EBP + -0x148);
      *(undefined4 *)(unaff_EBP + -0x124) = *(undefined4 *)(*(int *)(unaff_EBP + -0x19c) + 0x1658);
      while (*(int *)(unaff_EBP + -0x124) < *(int *)(*(int *)(unaff_EBP + -0x19c) + 0x1650)) {
        *(int *)(*(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4) + 0x3c +
                *(int *)(unaff_EBP + -0x124) * 0xfc) = *(int *)(unaff_EBP + -0x148) + -1;
        *(int *)(unaff_EBP + -0x124) = *(int *)(unaff_EBP + -0x124) + 1;
      }
      *(undefined4 *)
       (*(int *)(*(int *)(unaff_EBP + -0x19c) + 0x1668) + -4 + *(int *)(unaff_EBP + -0x148) * 4) =
           *(undefined4 *)(*(int *)(unaff_EBP + -0x19c) + 200);
    }
    _fclose(*(FILE **)(*(int *)(unaff_EBP + -0x19c) + 0x16d4));
    iVar3 = FUN_0042038b(*(int *)(unaff_EBP + -0x19c));
    *(int *)(*(int *)(unaff_EBP + -0x19c) + 0x166c) = iVar3;
    uVar2 = FUN_00422a3c(*(int *)(unaff_EBP + -0x19c));
    *(undefined4 *)(unaff_EBP + -0x144) = uVar2;
    if (*(int *)(unaff_EBP + -0x144) == 0) {
      uVar2 = FUN_004231b6(*(int *)(unaff_EBP + -0x19c));
      *(undefined4 *)(unaff_EBP + -0x144) = uVar2;
      if (*(int *)(unaff_EBP + -0x144) == 0) {
        *(undefined4 *)(unaff_EBP + -0x14c) = 1;
        *(undefined4 *)(unaff_EBP + -0x124) = *(undefined4 *)(*(int *)(unaff_EBP + -0x19c) + 0x1654)
        ;
        while (*(int *)(unaff_EBP + -0x124) < *(int *)(*(int *)(unaff_EBP + -0x19c) + 0x1658)) {
          if (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4) + 0x48 +
                      *(int *)(unaff_EBP + -0x124) * 0xfc) == 0) {
            *(undefined4 *)(unaff_EBP + -0x1a8) = *(undefined4 *)(unaff_EBP + -0x14c);
            FUN_0043ed39((char *)(*(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4) + 0x50 +
                                 *(int *)(unaff_EBP + -0x124) * 0xfc),&DAT_0044cbbc);
            *(int *)(unaff_EBP + -0x14c) = *(int *)(unaff_EBP + -0x14c) + 1;
          }
          *(int *)(unaff_EBP + -0x124) = *(int *)(unaff_EBP + -0x124) + 1;
        }
        *(undefined4 *)(*(int *)(unaff_EBP + -0x19c) + 0x234c) = *(undefined4 *)(unaff_EBP + -0x14c)
        ;
        if (*(int *)(unaff_EBP + -0x144) == 0) {
          if (DAT_00452e84 != 0) {
            *(undefined4 *)(unaff_EBP + -0x184) =
                 *(undefined4 *)(*(int *)(unaff_EBP + -0x19c) + 0x1658);
            while (*(int *)(unaff_EBP + -0x184) < *(int *)(*(int *)(unaff_EBP + -0x19c) + 0x1650)) {
              if (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4) + 0x48 +
                          *(int *)(unaff_EBP + -0x184) * 0xfc) == 0) {
                *(undefined4 *)(unaff_EBP + -0x17c) = 0;
                while ((*(uint *)(unaff_EBP + -0x17c) <
                        *(uint *)(*(int *)(unaff_EBP + -0x19c) + 200) &&
                       (iVar3 = _strcmp((char *)(*(int *)(unaff_EBP + -0x19c) + 0xd0 +
                                                *(int *)(unaff_EBP + -0x17c) * 9),
                                        (char *)(*(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4) + 4
                                                + *(int *)(unaff_EBP + -0x184) * 0xfc)), iVar3 != 0)
                       )) {
                  *(int *)(unaff_EBP + -0x17c) = *(int *)(unaff_EBP + -0x17c) + 1;
                }
                if (*(uint *)(*(int *)(unaff_EBP + -0x19c) + 200) <= *(uint *)(unaff_EBP + -0x17c))
                {
                  iVar3 = 0x1a0000;
                  goto LAB_00420009;
                }
                *(undefined4 *)(unaff_EBP + -0x128) = 0;
                while (*(uint *)(unaff_EBP + -0x128) < **(uint **)(unaff_EBP + -0x19c)) {
                  *(undefined4 *)(unaff_EBP + -0x118) = 0;
                  if (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x19c) + 0x84 +
                                       *(int *)(unaff_EBP + -0x17c) * 4) +
                              *(int *)(unaff_EBP + -0x128) * 4) != 2) {
                    *(undefined4 *)(unaff_EBP + -0x178) = 0;
                    while (*(int *)(unaff_EBP + -0x178) <
                           *(int *)(*(int *)(unaff_EBP + -0x19c) + 0x1654)) {
                      *(int *)(unaff_EBP + -0x174) =
                           1 << (((char)*(undefined4 *)(*(int *)(unaff_EBP + -0x19c) + 0x1654) + -1)
                                 - (char)*(undefined4 *)(unaff_EBP + -0x178) & 0x1fU);
                      if ((*(uint *)(unaff_EBP + -0x174) & *(uint *)(unaff_EBP + -0x128)) == 0) {
                        *(undefined4 *)
                         (*(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4) + 0x14 +
                         *(int *)(unaff_EBP + -0x178) * 0xfc) = 0;
                      }
                      else {
                        *(undefined4 *)
                         (*(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4) + 0x14 +
                         *(int *)(unaff_EBP + -0x178) * 0xfc) = 1;
                      }
                      *(int *)(unaff_EBP + -0x178) = *(int *)(unaff_EBP + -0x178) + 1;
                    }
                    iVar3 = FUN_00417769((void *)(*(int *)(unaff_EBP + -0x184) * 0xfc +
                                                 *(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4)),
                                         *(undefined4 *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4),
                                         *(undefined4 *)(unaff_EBP + -0x128));
                    *(int *)(unaff_EBP + -0x180) = iVar3;
                    if ((*(int *)(unaff_EBP + -0x180) < 0) || (1 < *(int *)(unaff_EBP + -0x180))) {
                      *(undefined4 *)(unaff_EBP + -0x118) = 0x1a;
                      break;
                    }
                    if ((*(int *)(unaff_EBP + -0x180) != 0) &&
                       (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x19c) + 0x84 +
                                         *(int *)(unaff_EBP + -0x17c) * 4) +
                                *(int *)(unaff_EBP + -0x128) * 4) == 0)) {
                      *(undefined4 *)(unaff_EBP + -0x118) = 0x21;
                      break;
                    }
                    if ((*(int *)(unaff_EBP + -0x180) == 0) &&
                       (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x19c) + 0x84 +
                                         *(int *)(unaff_EBP + -0x17c) * 4) +
                                *(int *)(unaff_EBP + -0x128) * 4) != 0)) {
                      *(undefined4 *)(unaff_EBP + -0x118) = 0x22;
                      break;
                    }
                    *(undefined4 *)(unaff_EBP + -0x124) = 0;
                    while (*(int *)(unaff_EBP + -0x124) <
                           *(int *)(*(int *)(unaff_EBP + -0x19c) + 0x1650)) {
                      FUN_0041770d((int *)(*(int *)(unaff_EBP + -0x124) * 0xfc +
                                          *(int *)(*(int *)(unaff_EBP + -0x19c) + 0x3a4)));
                      *(int *)(unaff_EBP + -0x124) = *(int *)(unaff_EBP + -0x124) + 1;
                    }
                  }
                  *(int *)(unaff_EBP + -0x128) = *(int *)(unaff_EBP + -0x128) + 1;
                }
              }
              *(int *)(unaff_EBP + -0x184) = *(int *)(unaff_EBP + -0x184) + 1;
            }
          }
          if (DAT_00452e8c != 0) {
            FID_conflict__fwprintf
                      (DAT_004528a8,(wchar_t *)"%d\t%d\t%d\n",
                       *(undefined4 *)(*(int *)(unaff_EBP + -0x19c) + 0x1650),
                       *(undefined4 *)(unaff_EBP + -0x118),
                       *(undefined4 *)(*(int *)(unaff_EBP + -0x19c) + 0x2308));
          }
          if (*(int *)(unaff_EBP + -0x118) == 0) {
            if (DAT_00452eb8 == 0) {
              FUN_00424347();
            }
            _free(*(void **)(unaff_EBP + -0x11c));
            *(undefined4 *)(unaff_EBP + -0x11c) = 0;
            iVar3 = 0;
          }
          else {
            iVar3 = *(int *)(unaff_EBP + -0x118) * 0x10000 + *(int *)(unaff_EBP + -0x128);
          }
        }
        else {
          _fclose(*(FILE **)(*(int *)(unaff_EBP + -0x19c) + 0x16d4));
          iVar3 = 0x1a0000;
        }
      }
      else {
        _fclose(*(FILE **)(*(int *)(unaff_EBP + -0x19c) + 0x16d4));
        iVar3 = 0x1a0000;
      }
    }
    else {
      _fclose(*(FILE **)(*(int *)(unaff_EBP + -0x19c) + 0x16d4));
      iVar3 = 0x1a0000;
    }
  }
LAB_00420009:
  ExceptionList = *(void **)(unaff_EBP + -0xc);
  return iVar3;
}
