#import <AppKit/AppKit.h>
#include <stdio.h>

@interface BaraGuiHelloWorldDelegate : NSObject <NSApplicationDelegate, NSWindowDelegate> {
    NSWindow *_window;
}
@end

@implementation BaraGuiHelloWorldDelegate

- (void)applicationDidFinishLaunching:(NSNotification *)notification {
    (void)notification;

    NSRect frame = NSMakeRect(200.0, 200.0, 360.0, 140.0);
    _window = [[NSWindow alloc]
        initWithContentRect:frame
                  styleMask:(NSWindowStyleMaskTitled | NSWindowStyleMaskClosable)
                    backing:NSBackingStoreBuffered
                      defer:NO];
    [_window setDelegate:self];
    [_window setTitle:@"Bara GUI Hello World"];

    NSTextField *label =
        [[NSTextField alloc] initWithFrame:NSMakeRect(20.0, 55.0, 320.0, 24.0)];
    [label setStringValue:@"hello world"];
    [label setEditable:NO];
    [label setBordered:NO];
    [label setDrawsBackground:NO];
    [label setAlignment:NSTextAlignmentCenter];
    [[_window contentView] addSubview:label];

    [NSApp activateIgnoringOtherApps:YES];
    [_window makeKeyAndOrderFront:nil];

    puts("{\"event\":\"gui_window_created\",\"title\":\"Bara GUI Hello World\",\"text\":\"hello world\"}");
    fflush(stdout);

#ifndef BARA_GUI_HELLO_WORLD_MANUAL_VISIBLE
    [NSTimer scheduledTimerWithTimeInterval:0.1
                                     target:self
                                   selector:@selector(terminateApplication:)
                                   userInfo:nil
                                    repeats:NO];
#endif
}

- (void)terminateApplication:(NSTimer *)timer {
    (void)timer;
    [NSApp terminate:nil];
}

- (void)windowWillClose:(NSNotification *)notification {
    (void)notification;
    [NSApp terminate:nil];
}

@end

int main(void) {
    @autoreleasepool {
        freopen("/dev/null", "w", stderr);

        [NSApplication sharedApplication];
        [NSApp setActivationPolicy:NSApplicationActivationPolicyRegular];

        BaraGuiHelloWorldDelegate *delegate =
            [[BaraGuiHelloWorldDelegate alloc] init];
        [NSApp setDelegate:delegate];
        [NSApp run];
    }

    return 0;
}
